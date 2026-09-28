# ADR-0010: Interruptible iterative search controller

- Status: Accepted
- Date: 2026-09-28

## Context

The M8 search could answer only a synchronous fixed-depth `go`. Tournament clients require the
engine to allocate clock time, accept `stop` while searching, provide useful progress, and retain a
legal move even when interrupted before the first full iteration. Tests also need a deterministic
resource boundary independent of machine speed.

## Decision

The search library retains its exact fixed-depth API and adds iterative deepening over depths one
through a configured maximum. A controller accepts optional cumulative node and wall-clock limits
plus a clonable atomic stop token. Principal and quiescence nodes check those limits before being
counted. Only fully completed iterations are published or eligible to replace the current result;
an interrupted partial PV is discarded. If depth one does not finish, the result carries a
deterministic legal fallback move.

Node counts are cumulative across iterations. `go nodes` therefore provides a reproducible hard
boundary, while time limits are inherently wall-clock dependent. Repetition history is supplied to
every iteration, preserving threefold, fivefold, 50-move, 75-move, and insufficient-material
semantics from M7.

The UCI adapter runs search on a scoped worker thread. The input loop remains available for
`stop`, `isready`, `quit`, new positions, and new searches. Output is serialized through a mutex;
the worker emits an `info` line after each completed iteration and exactly one `bestmove` when it
finishes. `info nodes` is cumulative and is accompanied by elapsed milliseconds and NPS.

`go movetime` reserves a small transmission margin. Clock allocation uses the side-to-move clock,
`movestogo` (default 30), and 75% of the matching increment, while retaining a reserve and never
allocating more than usable remaining time. Depth, node, and time limits compose: the first reached
limit stops the current iteration.

## Consequences

- UCI can remain responsive during arbitrarily deep searches without unsafe code or detached
  threads.
- Deterministic node limits make controller regressions reproducible.
- Wall-clock results and NPS vary by host, so tests use broad deadlines and exact node assertions
  rather than exact timing.
- The worker currently writes progress directly through synchronized protocol output. A future
  multi-search engine coordinator may replace this with typed event channels.
- Cancellation is cooperative at node boundaries; a single expensive node can add small latency.

## Verification

- Iterative results at every completed depth equal direct fixed-depth results.
- Tests prove exact and repeatable node exhaustion, cancellation before depth one, legal fallback,
  and preservation of draw-history scores.
- Parser tests cover depth, nodes, movetime, both clocks, both increments, and movestogo.
- A subprocess test starts depth 64, observes progress, checks `isready`, sends `stop`, and requires
  a prompt legal `bestmove`.
- Release-process transcripts exercise node, movetime, and clock-managed searches with `time` and
  `nps` fields.
