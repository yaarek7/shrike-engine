# ADR-0001: Library-first architecture and safe Rust baseline

- Status: Accepted
- Date: 2026-09-27

## Context

The engine will be developed through many automated experiments. Engine behavior must therefore
be testable and benchmarkable without terminal protocol orchestration. LLM-generated changes also
benefit from the strongest practical compile-time safety constraints.

## Decision

All engine behavior will live in a Rust library. The executable will remain a thin adapter around
that library. Unsafe Rust is forbidden at the crate and manifest levels during the initial
development stages.

## Alternatives considered

- Implement the engine directly in the executable.
- Permit unsafe Rust wherever an implementation appears performance-sensitive.
- Split the initial skeleton into multiple workspace crates.

## Consequences

- Unit tests, benchmarks, and future tools can call engine APIs directly.
- Protocol I/O cannot become entangled with core chess behavior without crossing an explicit
  architectural boundary.
- Potential unsafe optimizations require a later ADR, correctness proof strategy, and benchmark.
- A single crate avoids premature packaging complexity; workspace extraction remains possible.

## Verification

- An integration test consumes the public library API.
- The binary contains no engine implementation.
- CI denies compiler and Clippy warnings and builds the release target.
- Compilation fails if source code introduces an unsafe block.

