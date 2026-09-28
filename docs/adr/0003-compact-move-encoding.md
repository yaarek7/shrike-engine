# ADR-0003: Compact move encoding

- Status: Accepted
- Date: 2026-09-27

## Context

Moves will be created, sorted, copied, stored in transposition tables, and traversed throughout
search. Their representation should remain compact while preserving enough semantics to apply and
undo special moves without rediscovering their categories.

## Decision

A move is a `u16` value. Six bits encode the source square, six encode the destination square, and
four encode a move kind. Kinds distinguish quiet moves, pawn double pushes, both castlings,
ordinary captures, en-passant captures, and each promotion piece with and without capture.

Null moves are rejected by this representation. Search-level null moves will use an explicit,
separate control path. A move records its claimed semantics but does not prove that those semantics
are legal in a position; move generation and position application own that responsibility.

## Alternatives considered

- A larger struct containing source, destination, captured piece, and promotion fields.
- Inferring capture and special-move semantics whenever a move is applied.
- Reserving an ordinary encoded move as the search null move.

## Consequences

- Every move is exactly two bytes and inexpensive to copy.
- UCI coordinate notation can be produced without a position.
- UCI input will need to match coordinate text against generated legal moves to recover semantics.
- Captured-piece identity is not stored and must be obtained from the position when making a move.

## Verification

- The size of `Move` is asserted to be two bytes.
- Every supported kind round-trips through the raw encoding.
- Reserved kinds and same-square moves are rejected.
- Capture, promotion, and UCI-notation behavior is tested through the public API.

