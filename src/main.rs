//! Binary entry point for the chess engine.

#![forbid(unsafe_code)]

fn main() {
    // Protocol handling will be introduced as a library API in the UCI milestone.
    // Until then, the binary intentionally has no observable behavior.
    let _engine = chess_engine::EngineInfo::current();
}
