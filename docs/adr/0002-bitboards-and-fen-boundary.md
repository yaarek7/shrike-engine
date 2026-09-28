# ADR-0002: Bitboard mapping and FEN validation boundary

- Status: Accepted
- Date: 2026-09-27

## Context

Position representation will sit beneath move generation, evaluation, and search. Its coordinate
system must be stable before those systems are built. FEN parsing also needs a clear boundary
between malformed notation and chess-illegal positions.

## Decision

Piece locations are represented by twelve `u64` bitboards: one for each color and piece kind.
A1 maps to bit 0, files increase toward H, ranks increase toward rank 8, and H8 maps to bit 63.

Domain values such as colors, piece kinds, squares, pieces, and castling sides use distinct Rust
types. Other position state—side to move, castling rights, en-passant target, halfmove clock, and
fullmove number—is stored explicitly.

FEN parsing validates its six-field grammar, rank widths, piece symbols, active color, unique
castling symbols, en-passant target shape, and clock ranges. It does not yet reject positions for
chess-legality reasons such as absent kings, pawns on back ranks, impossible check states, or
inconsistent castling pieces. Those rules require attack and legal-move infrastructure and will be
introduced at that layer.

## Alternatives considered

- A 64-element mailbox board as the canonical representation.
- A redundant bitboard-plus-mailbox representation from the first milestone.
- Combining FEN syntax validation with incomplete chess-legality checks.

## Consequences

- Occupancies and piece sets are cheap to derive and suitable for future move generation.
- `piece_at` currently checks the twelve bitboards and is not intended as a search hot path.
- FEN can represent syntactically valid analysis fixtures that are not playable positions.
- Full legality validation remains an explicit future responsibility rather than a partial promise.

## Verification

- A1 and H8 bit mappings are tested.
- All 64 squares round-trip through algebraic notation.
- Canonical FEN fixtures round-trip exactly.
- Malformed fields and structural errors are rejected.
- The standard starting position contains the expected pieces and state.

