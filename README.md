# Chess Engine

An experimental chess engine built in Rust with a correctness-first, measurement-driven
development process.

## Status

The engine now has an explicit search composition boundary above the tournament controller.
`SearchBackend` implementations can be injected into UCI, while `AlphaBetaSearcher` owns a
replaceable evaluator and independently selectable quiescence and move-ordering policies. This
supports same-build A/B tests without virtual dispatch inside the node hot path.

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
