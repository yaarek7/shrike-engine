//! Universal Chess Interface (UCI) protocol session handling.

use std::{
    io::{self, BufRead, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    EngineInfo,
    chess::{Color, Move, Position, RepetitionKey},
    eval::ClassicalEvaluator,
    search::{
        AlphaBetaConfig, AlphaBetaSearcher, IterationInfo, SearchBackend, SearchLimits, StopToken,
    },
};

/// Runs a UCI session until end-of-input or the `quit` command.
///
/// Protocol state is kept entirely within this function, making sessions
/// deterministic and independently testable without spawning the executable.
///
/// # Errors
///
/// Returns an I/O error if reading a command or writing a response fails.
pub fn run<R: BufRead, W: Write + Send>(reader: &mut R, writer: &mut W) -> io::Result<()> {
    run_with_backend(
        reader,
        writer,
        Arc::new(AlphaBetaSearcher::new(
            ClassicalEvaluator,
            AlphaBetaConfig::default(),
        )),
    )
}

/// Runs a UCI session with an injected search backend.
///
/// This is the root composition boundary for same-build experiments.
///
/// # Errors
///
/// Returns an I/O error if reading a command, writing a response, or joining search fails.
pub fn run_with_backend<R: BufRead, W: Write + Send>(
    reader: &mut R,
    writer: &mut W,
    backend: Arc<dyn SearchBackend>,
) -> io::Result<()> {
    thread::scope(move |scope| {
        let output = Arc::new(Mutex::new(writer));
        let mut session = Session::default();
        let mut active: Option<(StopToken, thread::ScopedJoinHandle<'_, io::Result<()>>)> = None;
        let mut input = String::new();

        loop {
            if active
                .as_ref()
                .is_some_and(|(_, handle)| handle.is_finished())
            {
                if let Some(task) = active.take() {
                    join_search(task)?;
                }
            }
            input.clear();
            if reader.read_line(&mut input)? == 0 {
                if let Some(task) = active.take() {
                    task.0.stop();
                    join_search(task)?;
                }
                break;
            }
            let mut tokens = input.split_whitespace();
            let Some(command) = tokens.next() else {
                continue;
            };
            let remaining: Vec<_> = tokens.collect();
            match command {
                "uci" => with_output(&output, Session::identify)?,
                "isready" => with_output(&output, |writer| writeln!(writer, "readyok"))?,
                "position" => {
                    stop_and_join(&mut active)?;
                    if let Err(error) = session.set_position(&remaining) {
                        with_output(&output, |writer| {
                            writeln!(writer, "info string error: {error}")
                        })?;
                    }
                }
                "go" => {
                    stop_and_join(&mut active)?;
                    match parse_go(&remaining, session.position.side_to_move()) {
                        Ok(limits) => {
                            let position = session.position.clone();
                            let prior = session.prior_history().to_vec();
                            let stop = StopToken::default();
                            let worker_stop = stop.clone();
                            let worker_output = Arc::clone(&output);
                            let worker_backend = Arc::clone(&backend);
                            let handle = scope.spawn(move || {
                                run_search_worker(
                                    &position,
                                    &prior,
                                    limits,
                                    &worker_stop,
                                    worker_backend.as_ref(),
                                    &worker_output,
                                )
                            });
                            active = Some((stop, handle));
                        }
                        Err(error) => with_output(&output, |writer| {
                            writeln!(writer, "info string error: {error}")?;
                            writeln!(writer, "bestmove 0000")
                        })?,
                    }
                }
                "stop" | "ucinewgame" => stop_and_join(&mut active)?,
                "quit" => {
                    stop_and_join(&mut active)?;
                    break;
                }
                _ => {}
            }
        }
        with_output(&output, Write::flush)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Session {
    position: Position,
    history: Vec<RepetitionKey>,
}

impl Default for Session {
    fn default() -> Self {
        let position = Position::starting();
        let history = vec![position.repetition_key()];
        Self { position, history }
    }
}

impl Session {
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
        let mut history = vec![candidate.repetition_key()];

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
            history.push(candidate.repetition_key());
        }
        candidate.validate().map_err(|error| error.to_string())?;
        self.position = candidate;
        self.history = history;
        Ok(())
    }

    fn prior_history(&self) -> &[RepetitionKey] {
        self.history
            .strip_suffix(&[self.position.repetition_key()])
            .unwrap_or(&self.history)
    }
}

fn parse_go(tokens: &[&str], side: Color) -> Result<SearchLimits, String> {
    let mut depth = 64;
    let mut depth_specified = false;
    let mut infinite = false;
    let mut nodes = None;
    let mut move_time = None;
    let mut white_time = None;
    let mut black_time = None;
    let mut white_increment = 0_u64;
    let mut black_increment = 0_u64;
    let mut moves_to_go = 30_u64;
    let mut index = 0;
    while index < tokens.len() {
        let name = tokens[index];
        if name == "infinite" {
            infinite = true;
            index += 1;
            continue;
        }
        let recognized = matches!(
            name,
            "depth" | "nodes" | "movetime" | "wtime" | "btime" | "winc" | "binc" | "movestogo"
        );
        if !recognized {
            index += 1;
            continue;
        }
        let value = tokens
            .get(index + 1)
            .ok_or_else(|| format!("go {name} requires a value"))?;
        let number = value
            .parse::<u64>()
            .map_err(|_| format!("invalid {name} value '{value}'"))?;
        match name {
            "depth" => {
                depth_specified = true;
                depth = u8::try_from(number)
                    .map_err(|_| format!("search depth {number} is outside 1..=64"))?;
                if !(1..=64).contains(&depth) {
                    return Err(format!("search depth {depth} is outside 1..=64"));
                }
            }
            "nodes" => {
                if number == 0 {
                    return Err("go nodes must be positive".to_owned());
                }
                nodes = Some(number);
            }
            "movetime" => move_time = Some(number),
            "wtime" => white_time = Some(number),
            "btime" => black_time = Some(number),
            "winc" => white_increment = number,
            "binc" => black_increment = number,
            "movestogo" => {
                if number == 0 {
                    return Err("go movestogo must be positive".to_owned());
                }
                moves_to_go = number;
            }
            _ => unreachable!("recognized option"),
        }
        index += 2;
    }

    let time = move_time.map(movetime_budget).or_else(|| match side {
        Color::White => {
            white_time.map(|remaining| clock_budget(remaining, white_increment, moves_to_go))
        }
        Color::Black => {
            black_time.map(|remaining| clock_budget(remaining, black_increment, moves_to_go))
        }
    });
    if !depth_specified && nodes.is_none() && time.is_none() && !infinite {
        depth = 1;
    }
    Ok(SearchLimits { depth, nodes, time })
}

fn movetime_budget(milliseconds: u64) -> Duration {
    let margin = (milliseconds / 20).clamp(1, 10);
    Duration::from_millis(milliseconds.saturating_sub(margin))
}

fn clock_budget(remaining: u64, increment: u64, moves_to_go: u64) -> Duration {
    let reserve = (remaining / 20).clamp(1, 50);
    let usable = remaining.saturating_sub(reserve);
    let allocation = usable
        .checked_div(moves_to_go)
        .unwrap_or(0)
        .saturating_add(increment.saturating_mul(3) / 4)
        .min(usable);
    Duration::from_millis(allocation)
}

fn run_search_worker<W: Write>(
    position: &Position,
    prior: &[RepetitionKey],
    limits: SearchLimits,
    stop: &StopToken,
    backend: &dyn SearchBackend,
    output: &Arc<Mutex<&mut W>>,
) -> io::Result<()> {
    let mut output_error = None;
    let outcome = backend.iterative_search(position, prior, limits, stop, &mut |info| {
        if output_error.is_none() {
            output_error = with_output(output, |writer| write_search_info(writer, info)).err();
            if output_error.is_some() {
                stop.stop();
            }
        }
    });
    if let Some(error) = output_error {
        return Err(error);
    }
    match outcome {
        Ok(outcome) => with_output(output, |writer| match outcome.best_move() {
            Some(chess_move) => writeln!(writer, "bestmove {chess_move}"),
            None => writeln!(writer, "bestmove 0000"),
        }),
        Err(error) => with_output(output, |writer| {
            writeln!(writer, "info string error: {error}")?;
            writeln!(writer, "bestmove 0000")
        }),
    }
}

fn write_search_info<W: Write>(writer: &mut W, info: &IterationInfo) -> io::Result<()> {
    let result = info.result();
    write!(writer, "info depth {} score ", result.depth())?;
    if let Some(moves) = result.mate_in() {
        write!(writer, "mate {moves}")?;
    } else {
        write!(writer, "cp {}", result.score().centipawns())?;
    }
    let milliseconds = u64::try_from(info.elapsed().as_millis()).unwrap_or(u64::MAX);
    let nps = info
        .nodes()
        .saturating_mul(1_000)
        .checked_div(milliseconds.max(1))
        .unwrap_or(0);
    write!(
        writer,
        " nodes {} time {milliseconds} nps {nps}",
        info.nodes()
    )?;
    if !result.principal_variation().is_empty() {
        write!(writer, " pv")?;
        for chess_move in result.principal_variation() {
            write!(writer, " {chess_move}")?;
        }
    }
    writeln!(writer)
}

fn with_output<W: Write, T>(
    output: &Arc<Mutex<&mut W>>,
    operation: impl FnOnce(&mut W) -> io::Result<T>,
) -> io::Result<T> {
    let mut writer = output
        .lock()
        .map_err(|_| io::Error::other("UCI output lock was poisoned"))?;
    let result = operation(&mut **writer)?;
    writer.flush()?;
    Ok(result)
}

fn stop_and_join(
    active: &mut Option<(StopToken, thread::ScopedJoinHandle<'_, io::Result<()>>)>,
) -> io::Result<()> {
    if let Some(task) = active.take() {
        task.0.stop();
        join_search(task)?;
    }
    Ok(())
}

fn join_search(task: (StopToken, thread::ScopedJoinHandle<'_, io::Result<()>>)) -> io::Result<()> {
    task.1
        .join()
        .map_err(|_| io::Error::other("search worker panicked"))?
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
    use std::{
        collections::VecDeque,
        io::{self, BufRead, Read},
        sync::Arc,
        thread,
        time::Duration,
    };

    use super::{parse_go, run, run_with_backend};
    use crate::{
        chess::{Color, Move, Position},
        eval::{Evaluator, Score},
        search::{AlphaBetaConfig, AlphaBetaSearcher, QuiescencePolicy, SearchBackend},
    };

    fn transcript(input: &str) -> String {
        let mut reader = ScriptedReader {
            lines: input.split_inclusive('\n').map(str::to_owned).collect(),
            delay_next: false,
        };
        let mut output = Vec::new();
        run(&mut reader, &mut output).expect("in-memory I/O succeeds");
        String::from_utf8(output).expect("protocol output is UTF-8")
    }

    fn transcript_with_backend(input: &str, backend: Arc<dyn SearchBackend>) -> String {
        let mut reader = ScriptedReader {
            lines: input.split_inclusive('\n').map(str::to_owned).collect(),
            delay_next: false,
        };
        let mut output = Vec::new();
        run_with_backend(&mut reader, &mut output, backend).expect("in-memory I/O succeeds");
        String::from_utf8(output).expect("protocol output is UTF-8")
    }

    struct ScriptedReader {
        lines: VecDeque<String>,
        delay_next: bool,
    }

    impl Read for ScriptedReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Ok(0)
        }
    }

    impl BufRead for ScriptedReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            Ok(&[])
        }

        fn consume(&mut self, _amount: usize) {}

        fn read_line(&mut self, buffer: &mut String) -> io::Result<usize> {
            if self.delay_next {
                thread::sleep(Duration::from_millis(100));
                self.delay_next = false;
            }
            let Some(line) = self.lines.pop_front() else {
                return Ok(0);
            };
            self.delay_next = line.trim_start().starts_with("go");
            let length = line.len();
            buffer.push_str(&line);
            Ok(length)
        }
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
    fn uci_accepts_an_injected_search_composition() {
        struct ConstantEvaluator;

        impl Evaluator for ConstantEvaluator {
            fn evaluate(&self, _position: &Position) -> Score {
                Score::from_centipawns(123)
            }
        }

        let backend = AlphaBetaSearcher::new(
            ConstantEvaluator,
            AlphaBetaConfig {
                quiescence: QuiescencePolicy::Disabled,
                ..AlphaBetaConfig::default()
            },
        );
        let output =
            transcript_with_backend("position startpos\ngo depth 1\nquit\n", Arc::new(backend));

        assert!(output.contains("info depth 1 score cp -123 "));
        assert_legal_bestmove(&output, &Position::starting());
    }

    #[test]
    fn go_limits_parse_depth_nodes_and_time_controls() {
        let nodes = parse_go(&["nodes", "500"], Color::White).expect("valid nodes");
        assert_eq!(nodes.depth, 64);
        assert_eq!(nodes.nodes, Some(500));
        assert_eq!(nodes.time, None);

        let movetime = parse_go(&["movetime", "100"], Color::White).expect("valid movetime");
        assert_eq!(movetime.time, Some(Duration::from_millis(95)));

        let clocks = [
            "wtime",
            "30000",
            "btime",
            "9000",
            "winc",
            "1000",
            "binc",
            "200",
            "movestogo",
            "20",
        ];
        let white = parse_go(&clocks, Color::White).expect("valid clocks");
        let black = parse_go(&clocks, Color::Black).expect("valid clocks");
        assert_eq!(white.time, Some(Duration::from_millis(2_247)));
        assert_eq!(black.time, Some(Duration::from_millis(597)));
    }

    #[test]
    fn node_and_time_limited_go_commands_complete_reproducibly() {
        let output = transcript(concat!(
            "position startpos\n",
            "go nodes 500\n",
            "go nodes 500\n",
            "go movetime 20\n",
            "go wtime 100 btime 100 winc 5 binc 5 movestogo 20\n",
            "quit\n"
        ));
        let returned = bestmoves(&output);

        assert_eq!(returned.len(), 4);
        assert_eq!(returned[0], returned[1]);
        assert!(output.lines().any(|line| {
            line.starts_with("info depth ") && line.contains(" time ") && line.contains(" nps ")
        }));
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

        assert!(output.starts_with("info depth 1 score mate 0 nodes 1 time "));
        assert!(output.contains(" nps "));
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
    fn position_move_history_detects_threefold_repetition() {
        let position = Position::from_fen("4k3/8/8/8/8/8/7Q/4K3 b - - 0 1").expect("valid FEN");
        let output = transcript(concat!(
            "position fen 4k3/8/8/8/8/8/7Q/4K3 b - - 0 1 moves ",
            "e8f8 e1f1 f8e8 f1e1 ",
            "e8f8 e1f1 f8e8 f1e1\n",
            "go depth 3\n",
            "quit\n"
        ));

        assert!(output.contains("info depth 3 score cp 0 nodes "));
        assert_legal_bestmove(&output, &position);
    }

    #[test]
    fn claimable_fifty_move_draw_still_returns_a_legal_bestmove() {
        let position = Position::from_fen("4k3/8/8/8/8/8/7Q/4K3 b - - 100 51").expect("valid FEN");
        let output =
            transcript("position fen 4k3/8/8/8/8/8/7Q/4K3 b - - 100 51\ngo depth 2\nquit\n");

        assert!(output.contains("info depth 2 score cp 0 nodes "));
        assert_legal_bestmove(&output, &position);
    }

    #[test]
    fn stalemate_returns_null_bestmove() {
        let output = transcript("position fen 7k/5Q2/6K1/8/8/8/8/8 b - - 0 1\ngo\nquit\n");

        assert!(output.starts_with("info depth 1 score cp 0 nodes 1 time "));
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

        assert!(output.contains("info depth 2 score cp "));
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
                "info string error: invalid depth value 'nope'\n",
                "bestmove 0000\n",
                "readyok\n"
            )
        );
    }
}
