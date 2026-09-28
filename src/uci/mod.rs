//! Universal Chess Interface (UCI) protocol session handling.

use std::io::{self, BufRead, Write};

use crate::{
    EngineInfo,
    chess::{Move, Position},
    eval::ClassicalEvaluator,
    search::{SearchResult, search},
};

/// Runs a synchronous UCI session until end-of-input or the `quit` command.
///
/// Protocol state is kept entirely within this function, making sessions
/// deterministic and independently testable without spawning the executable.
///
/// # Errors
///
/// Returns an I/O error if reading a command or writing a response fails.
pub fn run<R: BufRead, W: Write>(reader: &mut R, writer: &mut W) -> io::Result<()> {
    let mut session = Session::default();
    let mut input = String::new();
    loop {
        input.clear();
        if reader.read_line(&mut input)? == 0 {
            break;
        }
        if !session.handle(input.trim(), writer)? {
            break;
        }
        writer.flush()?;
    }
    writer.flush()
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Session {
    position: Position,
}

impl Session {
    fn handle<W: Write>(&mut self, line: &str, writer: &mut W) -> io::Result<bool> {
        let mut tokens = line.split_whitespace();
        let Some(command) = tokens.next() else {
            return Ok(true);
        };

        match command {
            "uci" => Self::identify(writer)?,
            "isready" => writeln!(writer, "readyok")?,
            "position" => {
                let remaining: Vec<_> = tokens.collect();
                if let Err(error) = self.set_position(&remaining) {
                    writeln!(writer, "info string error: {error}")?;
                }
            }
            "go" => {
                let remaining: Vec<_> = tokens.collect();
                self.go(&remaining, writer)?;
            }
            "quit" => return Ok(false),
            // `ucinewgame` and `stop` have no stateful work in the synchronous
            // fixed-depth engine. Unknown commands are ignored for forward compatibility.
            _ => {}
        }
        Ok(true)
    }

    fn identify<W: Write>(writer: &mut W) -> io::Result<()> {
        let info = EngineInfo::current();
        writeln!(writer, "id name {} {}", info.name, info.version)?;
        writeln!(writer, "id author {}", info.author)?;
        writeln!(writer, "uciok")
    }

    fn set_position(&mut self, tokens: &[&str]) -> Result<(), String> {
        let mut index = 0;
        let mut candidate = match tokens.first().copied() {
            Some("startpos") => {
                index += 1;
                Position::starting()
            }
            Some("fen") => {
                if tokens.len() < 7 {
                    return Err("position fen requires six FEN fields".to_owned());
                }
                let fen = tokens[1..7].join(" ");
                index = 7;
                Position::from_fen(&fen).map_err(|error| error.to_string())?
            }
            _ => return Err("expected 'startpos' or 'fen'".to_owned()),
        };

        if index < tokens.len() {
            if tokens[index] != "moves" {
                return Err(format!("unexpected position token '{}'", tokens[index]));
            }
            index += 1;
        }

        for notation in &tokens[index..] {
            let chess_move = resolve_uci_move(&candidate, notation)?;
            candidate
                .make_move(chess_move)
                .map_err(|error| error.to_string())?;
        }
        candidate.validate().map_err(|error| error.to_string())?;
        self.position = candidate;
        Ok(())
    }

    fn go<W: Write>(&self, tokens: &[&str], writer: &mut W) -> io::Result<()> {
        let depth = match parse_depth(tokens) {
            Ok(depth) => depth,
            Err(error) => {
                writeln!(writer, "info string error: {error}")?;
                return writeln!(writer, "bestmove 0000");
            }
        };
        match search(&self.position, &ClassicalEvaluator, depth) {
            Ok(result) => write_search_result(writer, &result),
            Err(error) => {
                writeln!(writer, "info string error: {error}")?;
                writeln!(writer, "bestmove 0000")
            }
        }
    }
}

fn parse_depth(tokens: &[&str]) -> Result<u8, String> {
    let Some(index) = tokens.iter().position(|&token| token == "depth") else {
        return Ok(1);
    };
    let value = tokens
        .get(index + 1)
        .ok_or_else(|| "go depth requires a value".to_owned())?;
    let depth = value
        .parse::<u8>()
        .map_err(|_| format!("invalid search depth '{value}'"))?;
    if !(1..=64).contains(&depth) {
        return Err(format!("search depth {depth} is outside 1..=64"));
    }
    Ok(depth)
}

fn write_search_result<W: Write>(writer: &mut W, result: &SearchResult) -> io::Result<()> {
    write!(writer, "info depth {} score ", result.depth())?;
    if let Some(moves) = result.mate_in() {
        write!(writer, "mate {moves}")?;
    } else {
        write!(writer, "cp {}", result.score().centipawns())?;
    }
    write!(writer, " nodes {}", result.nodes())?;
    if !result.principal_variation().is_empty() {
        write!(writer, " pv")?;
        for chess_move in result.principal_variation() {
            write!(writer, " {chess_move}")?;
        }
    }
    writeln!(writer)?;
    match result.best_move() {
        Some(chess_move) => writeln!(writer, "bestmove {chess_move}"),
        None => writeln!(writer, "bestmove 0000"),
    }
}

fn resolve_uci_move(position: &Position, notation: &str) -> Result<Move, String> {
    position
        .legal_moves()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|chess_move| chess_move.to_uci() == notation)
        .ok_or_else(|| format!("illegal move '{notation}'"))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::run;
    use crate::chess::{Move, Position};

    fn transcript(input: &str) -> String {
        let mut reader = Cursor::new(input.as_bytes());
        let mut output = Vec::new();
        run(&mut reader, &mut output).expect("in-memory I/O succeeds");
        String::from_utf8(output).expect("protocol output is UTF-8")
    }

    fn bestmoves(output: &str) -> Vec<&str> {
        output
            .lines()
            .filter_map(|line| line.strip_prefix("bestmove "))
            .collect()
    }

    fn apply_uci(position: &mut Position, notation: &str) {
        let chess_move = position
            .legal_moves()
            .expect("test position is valid")
            .into_iter()
            .find(|chess_move| chess_move.to_uci() == notation)
            .expect("test move is legal");
        position.make_move(chess_move).expect("test move applies");
    }

    fn assert_legal_bestmove(output: &str, position: &Position) {
        let returned = bestmoves(output);
        assert_eq!(returned.len(), 1);
        let legal: Vec<String> = position
            .legal_moves()
            .expect("test position is valid")
            .into_iter()
            .map(Move::to_uci)
            .collect();
        assert!(legal.iter().any(|chess_move| chess_move == returned[0]));
    }

    #[test]
    fn handshake_and_readiness_follow_uci_framing() {
        let output = transcript("uci\nisready\nquit\n");
        let expected = format!(
            "id name Shrike Engine {}\nid author yaarek7\nuciok\nreadyok\n",
            env!("CARGO_PKG_VERSION")
        );

        assert_eq!(output, expected);
    }

    #[test]
    fn position_moves_are_applied_before_go() {
        let output = transcript("position startpos moves e2e4 e7e5\ngo depth 1\nquit\n");
        let mut expected = Position::starting();
        apply_uci(&mut expected, "e2e4");
        apply_uci(&mut expected, "e7e5");

        assert_legal_bestmove(&output, &expected);
    }

    #[test]
    fn fen_checkmate_returns_null_bestmove() {
        let output = transcript("position fen 7k/6Q1/6K1/8/8/8/8/8 b - - 0 1\ngo depth 1\n");

        assert!(output.starts_with("info depth 1 score mate 0 nodes 1\n"));
        assert!(output.ends_with("bestmove 0000\n"));
    }

    #[test]
    fn invalid_position_command_is_transactional() {
        let output = transcript(
            "position startpos moves e2e4\nposition startpos moves e2e5\ngo depth 1\nquit\n",
        );

        assert!(output.starts_with("info string error: illegal move 'e2e5'\n"));
        let mut expected = Position::starting();
        apply_uci(&mut expected, "e2e4");
        let bestmove_output = output
            .lines()
            .filter(|line| line.starts_with("bestmove "))
            .collect::<Vec<_>>()
            .join("\n");
        assert_legal_bestmove(&(bestmove_output + "\n"), &expected);
    }

    #[test]
    fn new_game_does_not_replace_the_position() {
        let output = transcript("position startpos moves e2e4\nucinewgame\ngo depth 1\nquit\n");
        let mut expected = Position::starting();
        apply_uci(&mut expected, "e2e4");

        assert_legal_bestmove(&output, &expected);
    }

    #[test]
    fn special_moves_in_position_commands_are_resolved_by_legality() {
        let output = transcript(concat!(
            "position fen r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1 moves e1g1\n",
            "go depth 1\n",
            "position fen 7k/8/8/3pP3/4K3/8/8/8 w - d6 0 1 moves e5d6\n",
            "go depth 1\n",
            "position fen 4k3/P7/8/8/8/8/8/4K3 w - - 0 1 moves a7a8q\n",
            "go depth 1\n",
            "quit\n"
        ));

        assert!(!output.contains("info string error"));
        assert_eq!(bestmoves(&output).len(), 3);
    }

    #[test]
    fn repeated_go_does_not_mutate_the_position_and_stop_is_silent() {
        let output = transcript("position startpos\ngo depth 1\nstop\ngo depth 1\nquit\n");
        let returned = bestmoves(&output);

        assert_eq!(returned.len(), 2);
        assert_eq!(returned[0], returned[1]);
    }

    #[test]
    fn stalemate_returns_null_bestmove() {
        let output = transcript("position fen 7k/5Q2/6K1/8/8/8/8/8 b - - 0 1\ngo\nquit\n");

        assert!(output.starts_with("info depth 1 score cp 0 nodes 1\n"));
        assert!(output.ends_with("bestmove 0000\n"));
    }

    #[test]
    fn malformed_fen_does_not_poison_the_session() {
        let output =
            transcript("position fen 8/8/8/8/8/8/8/8 w - - 0 1\nisready\ngo depth 1\nquit\n");

        assert!(output.starts_with("info string error: White king is missing\nreadyok\n"));
        assert_legal_bestmove(&output, &Position::starting());
    }

    #[test]
    fn whitespace_unknown_commands_and_quit_are_handled_safely() {
        let output = transcript("\r\nunknown future command\r\nisready\r\nquit\r\nisready\r\n");

        assert_eq!(output, "readyok\n");
    }

    #[test]
    fn go_reports_requested_depth_score_nodes_and_pv() {
        let output = transcript("position fen 4k3/8/8/8/8/8/q7/R3K3 w - - 0 1\ngo depth 2\nquit\n");

        assert!(output.starts_with("info depth 2 score cp "));
        assert!(output.contains(" nodes "));
        assert!(output.contains(" pv a1a2 "));
        assert!(output.ends_with("bestmove a1a2\n"));
    }

    #[test]
    fn malformed_depth_completes_with_null_move() {
        let output = transcript("go depth nope\nisready\nquit\n");

        assert_eq!(
            output,
            concat!(
                "info string error: invalid search depth 'nope'\n",
                "bestmove 0000\n",
                "readyok\n"
            )
        );
    }
}
