# ADR-0007: Deterministic fixed-depth alpha-beta search

- Status: Accepted
- Date: 2026-09-28

## Context

The engine needs a first real move-selection algorithm that is small enough to verify, exposes
useful measurements, and leaves room for later iterative deepening, time management, and
transposition tables. It must preserve the correctness boundary established by legal move
generation and make/unmake, while avoiding repeated validation at every search node.

## Decision

The baseline search is depth-limited negamax with alpha-beta pruning. The public entry point
validates and clones the root once; descendants use crate-private legal generation and
make/unmake operations whose invariants are established by the validated root and legal moves.
The current correctness-first undo record snapshots the position at every move, so search still
incurs per-node cloning internally. The evaluator is injected through the existing trait.

Terminal detection precedes the depth cutoff so checkmate and stalemate are recognized at the
horizon. Mate scores occupy a band around 900,000 centipawns and encode distance by ply; static
evaluation is clamped to 100,000 centipawns so it cannot collide with that band. Captures and
promotions are searched first, with the compact move encoding as a deterministic tie-breaker.
The result reports the root-relative score, root-inclusive node count, and principal variation.

The UCI adapter supports `go depth N` for depths 1 through 64 and defaults to depth one when no
depth is supplied. Each completed search emits one `info` line followed by exactly one `bestmove`.

## Consequences

- Search behavior is deterministic and directly testable without protocol I/O.
- Alpha-beta pruning can be measured against an unpruned reference while producing the same score.
- Make/unmake centralizes state restoration, but the current full-position undo snapshot still
  clones at each searched move. Compact delta-based undo is deferred until profiling work.
- The baseline has no quiescence search, iterative deepening, time control, repetition detection,
  transposition table, or sophisticated ordering; those remain explicit future milestones.
- Principal variations allocate vectors at nodes. This is acceptable for the correctness baseline
  and provides a clear target for later profiling and optimization.

## Verification

- Unit tests compare the score with an unpruned reference and require fewer visited nodes.
- Tactical tests cover a free queen, mate in one, checkmate, and stalemate.
- Tests verify side-to-move score semantics, deterministic repetition, legal PV replay, evaluator
  clamping, root preservation, and invalid-root rejection.
- Integration tests exercise the public search API and UCI tests exercise `go depth` framing.
- Release-mode UCI measurements and the full quality gate are recorded in `docs/verification/M6.md`.
