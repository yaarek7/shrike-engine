# Architecture

## Purpose

This repository is both a chess engine and an experimental platform. Its architecture must
support correctness testing, performance measurement, engine-versus-engine comparisons, and
eventual replacement of classical components with learned ones.

## Current boundary

The package produces:

- `chess_engine`, a library that owns all reusable engine behavior;
- `chess-engine`, a thin executable that will eventually adapt standard input and output to the
  library's UCI protocol API.

M2 adds a compact, semantic move value to the foundational `chess` module. The module now contains
strongly typed domain values, twelve piece bitboards, complete FEN position state, strict FEN
syntax parsing, and two-byte moves. It intentionally does not yet include attack detection,
position mutation, or move generation.

## Intended module progression

Modules will be introduced only when their milestone begins:

```text
src/
├── lib.rs
├── main.rs
├── chess/       # Position and FEN now; moves and attacks in later milestones
├── uci/         # Protocol parsing and session handling
├── eval/        # Replaceable evaluation implementations
├── search/      # Search, ordering, time management, transposition table
└── engine.rs    # Coordination without protocol-specific I/O
```

Empty speculative modules are avoided. Public APIs should emerge from concrete use cases and
tests rather than anticipated requirements.

## Board representation

The board uses A1 as bit 0 and H8 as bit 63. Position piece placement is held in one bitboard per
color and piece kind. See ADR-0002 for the representation and FEN validation boundary.

## Dependency direction

The protocol layer may depend on the engine API. The engine may depend on chess, evaluation,
and search abstractions. Core chess representation must not depend on UCI, evaluation, or search.

```text
binary → UCI → engine → search → evaluation
                         ↓
                       chess
```

## Invariants

1. The library can be tested without launching the executable.
2. Core chess logic does not perform terminal or file I/O.
3. Search and evaluation remain independently replaceable and testable.
4. Deterministic execution remains available for tests and experiments.
5. No unsafe Rust is permitted without a measured need and an accepted ADR.
6. Hot-path allocations will be measured and controlled once hot paths exist.

## Decision records

Material architectural decisions are recorded under `docs/adr/`. An ADR describes context,
the selected decision, alternatives, consequences, and verification criteria.
