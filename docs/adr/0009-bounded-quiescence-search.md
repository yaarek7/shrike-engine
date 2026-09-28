# ADR-0009: Bounded quiescence search

- Status: Accepted
- Date: 2026-09-28

## Context

The fixed-depth search evaluated every nominal leaf immediately. That exposes a horizon effect:
the evaluator can reward a capture before an obvious recapture is searched, and it can evaluate a
side while that side is in check. Search needs a tactical stabilization step without introducing
time management, selective pruning heuristics, or a transposition table in the same milestone.

## Decision

Nominal depth zero enters alpha-beta quiescence search. At a node not in check, the static
evaluation is the stand-pat score and only legal captures and promotions are searched. Promotions
include quiet promotions, and captures include en passant. At a node in check, stand pat is not a
legal option and every legal evasion is searched. The existing deterministic tactical ordering is
used throughout.

Legal-move terminal detection and M7 draw semantics run before static evaluation. Claimable draws
remain a zero-valued option, while automatic draws terminate the node. Checkmate scores continue
to encode distance from the root across quiescence plies.

Quiescence is bounded to 32 recursive plies. At that guard, after terminal and draw detection, a
side that is not in check returns the bounded static evaluation. A checked side never stands pat:
search examines its legal evasions through one final finite ply, then applies terminal and draw
precedence before statically evaluating each child without further recursion. This explicit safety
limit prevents a checking-evasion cycle from making fixed-depth search unbounded without scoring
the checked position as though passing were legal. The mate-recognition band covers the maximum 64
nominal plies, 32 recursive quiescence plies, and the final capped evasion ply.

`SearchResult::nodes` counts all visited nodes exactly once and
`SearchResult::quiescence_nodes` reports the quiescence subset. Principal variations contain only
nominal-depth moves; quiescence moves influence scores and move choice but are intentionally not
reported. The UCI `nodes` field remains the standard total-node measurement. The library accessor
provides the diagnostic quiescence split without adding a non-standard UCI token.

## Consequences

- Shallow search no longer prefers simple poisoned captures that are refuted by recapture.
- Checked leaves search legal evasions instead of applying stand pat.
- Search remains deterministic and bounded for tests and protocol callers.
- Total node counts are not directly comparable with pre-M8 counts because tactical leaves now
  expand; quiescence nodes make that additional work measurable.
- The 32-ply guard plus one final checked evasion is a correctness-versus-termination compromise.
  Repetition handling normally stops real cycles earlier, but the guard remains a final safety
  boundary.
- Static exchange evaluation, delta pruning, checks from otherwise quiet positions, and learned
  move ordering remain future, independently measurable changes.

## Verification

- Tactical tests cover a poisoned capture, a forced recapture, quiet promotion, en passant, and
  quiet check evasions.
- Tests cover checkmate and stalemate at the horizon, mate found through a quiescence capture, and
  direct exercise of every legal evasion at the extension guard.
- Capture promotions exercise queen, rook, bishop, and knight promotion kinds.
- An unpruned quiescence oracle must return the same score while alpha-beta visits fewer nodes.
- Public tests require deterministic results, preserved roots, legal replayable nominal PVs, and
  consistent total/quiescence node accounting.
