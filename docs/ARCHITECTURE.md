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

M8 adds bounded quiescence search over captures, promotions, and all legal check evasions. It
stabilizes tactical leaf evaluation while retaining deterministic search, total and quiescence
node measurements, and nominal-depth principal variations. M7 provides deterministic incremental
repetition keys and draw-aware search using explicit game history, the halfmove clock, and
conservative insufficient-material recognition. M6 provides fixed-depth negamax search with
alpha-beta pruning, tactical move ordering, mate-distance scores, node counts, and principal
variations. M5 provides its replaceable classical evaluator, and M4 provides the synchronous,
library-testable UCI boundary and functional stdin/stdout executable. The `chess` module contains
strongly typed domain values, twelve piece bitboards, complete FEN position state, strict FEN
syntax parsing, two-byte moves, attack detection, legal move generation, validated make/unmake,
perft, and FIDE-relevant repetition identity. Protocol, evaluation, and search remain outside the
chess-rules layer.

## Intended module progression

Modules will be introduced only when their milestone begins:

```text
src/
├── lib.rs
├── main.rs
├── chess/       # Position, FEN, moves, attacks, legal generation, and perft
├── uci/         # Synchronous protocol parsing and session state
├── eval/        # Classical baseline behind a replaceable evaluator trait
├── search/      # Fixed-depth negamax, alpha-beta, quiescence, and move ordering
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
binary → UCI → search → evaluation
          ↓       ↓
        chess ←─────┘
```

## Invariants

1. The library can be tested without launching the executable.
2. Core chess logic does not perform terminal or file I/O.
3. Search and evaluation remain independently replaceable and testable.
4. Deterministic execution remains available for tests and experiments.
5. No unsafe Rust is permitted without a measured need and an accepted ADR.
6. Hot-path allocations will be measured and controlled once hot paths exist.
7. Repetition history is explicit search input and remains conceptually separate from future
   transposition-table identity and storage.

## Decision records

Material architectural decisions are recorded under `docs/adr/`. An ADR describes context,
the selected decision, alternatives, consequences, and verification criteria.
