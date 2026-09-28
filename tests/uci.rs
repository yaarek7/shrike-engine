//! End-to-end tests for the UCI executable.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use chess_engine::chess::{Move, Position};

#[test]
fn executable_completes_a_uci_session() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_chess-engine"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("engine executable starts");

    child
        .stdin
        .take()
        .expect("child stdin is piped")
        .write_all(b"uci\nisready\nposition startpos\ngo depth 1\nquit\n")
        .expect("protocol input is written");
    let output = child.wait_with_output().expect("engine exits");
    let stdout = String::from_utf8(output.stdout).expect("engine output is UTF-8");

    assert!(output.status.success());
    assert!(stdout.contains("uciok\n"));
    assert!(stdout.contains("readyok\n"));
    let bestmove = stdout
        .lines()
        .find_map(|line| line.strip_prefix("bestmove "))
        .expect("engine emits bestmove");
    let legal: Vec<String> = Position::starting()
        .legal_moves()
        .expect("start position is valid")
        .into_iter()
        .map(Move::to_uci)
        .collect();
    assert!(legal.iter().any(|chess_move| chess_move == bestmove));
}
