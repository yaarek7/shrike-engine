# ADR-0008: Repetition identity and draw-aware search

- Status: Accepted
- Date: 2026-09-28

## Context

Search previously treated only checkmate and stalemate as terminal outcomes. Correct game-tree
scores also require game history for repeated positions, while the existing FEN position already
contains the halfmove clock needed for the fifty-move rule. Future transposition tables will also
need hashing, but repetition and transposition-table identity have different lifecycle and draw
semantics and should not be coupled prematurely.

## Decision

Each `Position` stores a deterministic 64-bit Zobrist `RepetitionKey`. Piece placement, side to
move, and castling permissions contribute to the key. An en-passant file contributes only when
the side to move has a legal en-passant capture, including the king-safety constraint. Halfmove
and fullmove clocks do not contribute because they are not part of FIDE repetition identity.

Move application updates the key incrementally, and the existing full-position undo snapshot
restores it with the rest of the position. A full recomputation exists inside the chess module as
an independent correctness oracle. Zobrist values are generated from fixed indices and a fixed
SplitMix64 seed, so builds and platforms produce the same keys.

The existing `search(position, evaluator, depth)` API remains history-free and compatible.
`search_with_history` accepts repetition keys for positions preceding the root; search appends
and removes descendant keys alongside make/unmake. UCI `position` processing records the initial
position and every applied move, then passes only the keys preceding its current root.

At each node, search first checks for legal moves so checkmate and stalemate retain precedence.
A third occurrence or a halfmove clock of at least 100 plies creates a claimable zero-valued
option. Search continues from that node and selects a better continuation when one exists; the
claim therefore behaves as a score floor rather than a forced terminal. A fifth occurrence or a
halfmove clock of at least 150 plies is an automatic draw and stops search at that node.

A conservative insufficient-material detector is also an automatic draw. It is limited to king
versus king, a lone bishop or knight versus king, and bishops-only positions with all bishops on
one square color. It is not presented as complete FIDE dead-position detection.

UCI has no command by which an engine claims a draw. Therefore every root with legal moves still
returns a deterministic legal `bestmove`, including when a claim or automatic draw supplies the
selected zero score. In that case the principal variation is empty because the fallback move was
not the source of the score. Checkmate and stalemate alone return no move.

## Consequences

- Repetition is evaluated using actual game history instead of reconstructing it from FEN.
- Raw FENs that differ only by an uncapturable en-passant target share repetition identity.
- A history-free caller still receives move-clock claim and automatic-draw semantics plus
  conservative material draws, but cannot receive repetition semantics unless it calls the
  history-aware API.
- Winning continuations, including checkmate, outrank an available draw claim. A losing side can
  retain a zero score from the claim while still supplying a legal move to the protocol boundary.
- The repetition key must not silently become a future transposition-table key. In particular,
  the halfmove clock is excluded here but can affect search value through the fifty-move rule; a
  transposition table must account for that context in its lookup policy.
- Zobrist collisions are possible in principle. Full 64-bit keys are compared, which makes the
  practical risk negligible for in-search repetition histories but does not make them proofs.

## Verification

- Unit tests compare every incrementally updated key with full recomputation over normal,
  castling, en-passant, capture, and promotion moves, then verify exact restoration by unmake.
- Key tests cover side-to-move, castling, capturable, uncapturable, and pinned en-passant state,
  plus clock exclusion.
- Search tests cover claimable threefold and fifty-move draws, automatic fivefold and 75-move
  draws, conservative material draws, winning continuations over claims, and checkmate precedence.
- UCI tests build a threefold repetition exclusively through `position ... moves`.
- Public integration tests exercise the history-aware search API.
