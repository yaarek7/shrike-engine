# Chess Engine

An experimental chess engine built in Rust with a correctness-first, measurement-driven
development process.

## Status

Milestone M6 is complete: the engine now selects moves with deterministic, fixed-depth negamax
search, alpha-beta pruning, simple tactical move ordering, mate-distance scores, and principal
variation reporting. The UCI process accepts `go depth N` and reports score, node count, principal
variation, and best move.

## Requirements

- Rust 1.85 or newer
- Cargo

On macOS with Homebrew:

```sh
brew install rust
```

## Local checks

Run the same quality gates used by CI:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
cargo build --release --locked
```

Apply formatting with:

```sh
cargo fmt --all
```

## Engineering principles

- Correctness is established through tests, not inspection alone.
- Performance and playing-strength claims require measurements.
- Each experiment changes one independent variable.
- The binary remains thin; engine behavior belongs in testable library modules.
- Unsafe Rust is forbidden until a future benchmark and architecture decision justify it.
- Architectural decisions and experimental outcomes are recorded in the repository.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) and
[`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) for the working agreements.
