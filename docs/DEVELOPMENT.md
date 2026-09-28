# Development

## Definition of done

A change is complete only when:

1. Its behavior and scope are explicit.
2. Relevant automated tests exist and pass.
3. Formatting and Clippy checks pass without warnings.
4. Public interfaces have useful documentation.
5. Performance-sensitive changes include reproducible before/after measurements.
6. Chess-strength claims include controlled engine-versus-baseline results.
7. Architectural changes include an ADR.

## Change discipline

- Keep each change focused on one behavior or hypothesis.
- Do not modify search and evaluation in the same experiment.
- Do not alter move generation without running the complete perft suite once it exists.
- Do not mix unrelated refactoring with behavioral changes.
- Prefer a clear implementation until profiling identifies a measured bottleneck.
- Preserve a deterministic mode for debugging and comparisons.

## Required checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
cargo build --release --locked
```

CI runs all five checks. The lockfile is part of the application build and should remain current.

## Testing layers

- Unit tests belong beside small implementation units.
- Integration tests under `tests/` validate public behavior and module boundaries.
- Perft tests will become the correctness oracle for move generation.
- Benchmarks will track speed independently from playing strength.
- Engine matches will evaluate playing strength independently from raw speed.

## Experiments

Each strength or performance experiment should eventually record:

- hypothesis;
- baseline and candidate revisions;
- correctness-test results;
- benchmark environment and results;
- match setup and statistical result;
- accept/reject decision.
