# ADR-0004: Correctness-first legal move generation

- Status: Accepted
- Date: 2026-09-27

## Context

Legal move generation is the correctness foundation for every later search and evaluation result.
Special moves, pins, checks, and state restoration create a large interacting bug surface. The
first implementation should make those interactions observable and testable before optimizing
them.

## Decision

Move generation first creates pseudo-legal moves by piece type, then makes each candidate and
rejects it when the moving side's king is attacked. Castling additionally checks the initial,
transit, and destination squares. En-passant application removes the captured pawn before king
safety is evaluated.

Public move application accepts only a move present in the generated legal list. Internal
unchecked application is crate-private and is used only after move generation. Undo currently
stores a complete position snapshot, favoring exact restoration over minimal state size.

FEN remains a syntax parser. A separate position validation boundary checks the invariants needed
for legal move generation: one king per color, no pawns on back ranks, nonadjacent kings, reachable
check state, and consistent castling and en-passant metadata.

## Alternatives considered

- Generate only legal moves using pins and check masks from the outset.
- Expose unchecked move application publicly.
- Use a compact incremental undo record immediately.
- Treat inconsistent FEN metadata as absent rather than reporting an invalid position.

## Consequences

- The algorithm is straightforward to verify against perft and targeted rule fixtures.
- Public callers cannot accidentally apply a pseudo-legal or incorrectly classified move.
- Full-position undo records are larger than necessary, but avoid partial-restoration defects.
- Move generation scans bitboards and allocates a vector; these are explicit future optimization
  targets only after profiling.

## Verification

- All six canonical Chess Programming Wiki perft positions match through depth four.
- The starting position and rook/pawn endgame match through depth five in the routine suite.
- A retained release-mode deep suite verifies start depth six and four additional depth-five
  counts.
- Targeted tests cover castling through check, pinned en passant, en-passant check evasion, double
  check, all promotions, castling-right revocation, clocks, and exact make/unmake restoration.

