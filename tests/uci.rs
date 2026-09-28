//! End-to-end tests for the UCI executable.

use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
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

#[test]
fn stop_interrupts_a_running_process_and_emits_one_bestmove() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_chess-engine"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("engine executable starts");
    let mut stdin = child.stdin.take().expect("child stdin is piped");
    let stdout = child.stdout.take().expect("child stdout is piped");
    let (sender, receiver) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if sender
                .send(line.expect("engine output is readable"))
                .is_err()
            {
                break;
            }
        }
    });

    writeln!(stdin, "uci").expect("uci command is written");
    stdin.flush().expect("uci command is flushed");
    receive_until(&receiver, Duration::from_secs(2), |line| line == "uciok");

    writeln!(stdin, "position startpos").expect("position command is written");
    writeln!(stdin, "go depth 64").expect("go command is written");
    stdin.flush().expect("search commands are flushed");
    let first_info = receive_until(&receiver, Duration::from_secs(3), |line| {
        line.starts_with("info depth ")
    });
    assert!(first_info.contains(" time "));
    assert!(first_info.contains(" nps "));

    writeln!(stdin, "isready").expect("readiness command is written");
    stdin.flush().expect("readiness command is flushed");
    receive_until(&receiver, Duration::from_secs(2), |line| line == "readyok");

    let stop_started = Instant::now();
    writeln!(stdin, "stop").expect("stop command is written");
    stdin.flush().expect("stop command is flushed");
    let bestmove = receive_until(&receiver, Duration::from_secs(3), |line| {
        line.starts_with("bestmove ")
    });
    assert!(stop_started.elapsed() < Duration::from_secs(3));
    assert_ne!(bestmove, "bestmove 0000");

    writeln!(stdin, "quit").expect("quit command is written");
    stdin.flush().expect("quit command is flushed");
    drop(stdin);
    let status = child.wait().expect("engine exits");
    reader.join().expect("reader thread exits");
    assert!(status.success());
    assert_eq!(
        receiver
            .try_iter()
            .filter(|line| line.starts_with("bestmove "))
            .count(),
        0,
        "stop must produce exactly one bestmove"
    );
}

fn receive_until(
    receiver: &mpsc::Receiver<String>,
    timeout: Duration,
    predicate: impl Fn(&str) -> bool,
) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let line = receiver
            .recv_timeout(remaining)
            .expect("expected UCI output before timeout");
        if predicate(&line) {
            return line;
        }
    }
}
