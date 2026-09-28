//! Binary entry point for the chess engine.

#![forbid(unsafe_code)]

use std::{io, process::ExitCode};

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut writer = io::BufWriter::new(io::stdout());

    match chess_engine::uci::run(&mut reader, &mut writer) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("UCI I/O error: {error}");
            ExitCode::FAILURE
        }
    }
}
