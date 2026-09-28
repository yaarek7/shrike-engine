# Search controller verification

- Date: 2026-09-28
- Build profile for empirical runs: `release`

## Deterministic checks

- Every iterative depth through three matches the corresponding direct fixed-depth result.
- A one-node budget completes no iteration, visits exactly one node, and returns a legal fallback.
- Repeated 500-node searches stop at exactly 500 nodes with identical completed result and move.
- A pre-cancelled search visits zero nodes and returns a legal fallback.
- History-aware iterative search retains claimable repetition and 50-move draw scoring.

## Process checks

The executable process test starts `go depth 64`, waits for an `info` line, confirms `time` and
`nps`, sends `isready` while search is active, then sends `stop`. It requires `readyok` and one
non-null `bestmove` within a generous three-second deadline.

Release transcripts additionally produced:

| Command | Last completed depth | Best move | Observed behavior |
| --- | ---: | --- | --- |
| `go nodes 1000` | 2 | `e2e4` | hard node limit interrupted the next iteration |
| `go movetime 100` | 4 | `e2e4` | completed in the allocated 95 ms budget |
| `go wtime 1000 btime 1000 winc 0 binc 0 movestogo 20` | 4 | `e2e4` | used the side-to-move clock allocation |

All completed iterations included cumulative `nodes`, elapsed `time`, computed `nps`, score, and
principal variation. A manually stopped depth-64 search returned the last completed iteration's
legal move.

## Quality gate

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo test --release --test perft --locked -- --nocapture
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
cargo build --release --locked
git diff --check
```
