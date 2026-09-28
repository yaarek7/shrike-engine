# ADR-0005: Synchronous UCI boundary

- Status: Accepted
- Date: 2026-09-27

## Context

The engine needs a standard process protocol before search exists. Protocol behavior must remain
testable without process orchestration, while the executable should contain no chess logic.

## Decision

UCI session handling is a generic library function over `BufRead` and `Write`. The executable only
locks standard input/output and delegates to that function. Position commands are transactional:
the session position changes only after the FEN and every supplied UCI move validate successfully.
`ucinewgame` is treated as a cache/history boundary and is currently a no-op; it does not replace
the position command required by the protocol.

UCI coordinate input is resolved against the generated legal moves. This recovers semantic move
kinds—capture, castling, en passant, and promotion—that coordinate text alone cannot determine.

Until search is introduced, `go` deterministically returns the first legal move, or `0000` when
there is no legal move. `stop` is accepted as a no-op because M4 performs no asynchronous work.

## Consequences

- Protocol tests use in-memory I/O and an end-to-end executable test.
- Malformed `position` input cannot partially mutate session state.
- The engine can communicate with a UCI GUI but does not yet choose moves intelligently.
- Time controls, options, pondering, and asynchronous stop behavior remain future work.

## Verification

- Exact handshake and readiness transcripts are tested.
- Start-position and six-field FEN commands are accepted.
- Sequential UCI moves are legality-checked and applied.
- Invalid position commands preserve the previous session position.
- Checkmate emits `bestmove 0000`.
- The compiled executable completes a piped UCI session successfully.
