# Testing Shrike

This guide covers the automated checks and Fastchess comparison workflow used to validate and
estimate the playing strength of a frozen Shrike build. Generated binaries, opening books, logs,
and PGNs belong under `matches/`, which is intentionally ignored by Git because an opening suite
alone can exceed 200 MB.

## Environment

The verified macOS setup uses Apple Silicon, Homebrew Stockfish, and a locally built Fastchess:

```sh
brew install stockfish

mkdir -p /Users/yaarek/projects/tools
git clone --depth 1 --branch v1.8.2-alpha \
  https://github.com/Disservin/fastchess.git \
  /Users/yaarek/projects/tools/fastchess
make -C /Users/yaarek/projects/tools/fastchess -j CXX=clang++

brew list --versions stockfish
/Users/yaarek/projects/tools/fastchess/fastchess -version
```

The verified versions were Stockfish 19 and Fastchess alpha 1.8.2 at commit `f618e34`.

## Repository checks

Run the complete quality gate before comparing engine strength:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features --locked
cargo build --release --locked
```

Changes to chess state or move generation also require release perft:

```sh
cargo test --release --test perft --locked -- --nocapture
```

## Freeze the tested engine

Never rebuild an engine during a match series. Copy the release executable and record both its
revision and checksum:

```sh
mkdir -p matches/bin
git rev-parse HEAD > matches/engine-revision.txt
cp target/release/chess-engine matches/bin/shrike-current
shasum -a 256 matches/bin/shrike-current > matches/engine-sha256.txt
```

Give each frozen executable a unique revision-based name before retaining results from multiple
versions.

## UCI compliance

Run Fastchess's compliance check against the exact frozen executable:

```sh
/Users/yaarek/projects/tools/fastchess/fastchess --compliance \
  /Users/yaarek/projects/chess/matches/bin/shrike-current
```

The tournament-controller baseline passed all 40 checks, including clock and increment handling.
A failure here must be resolved before interpreting match results.

## Opening suite

Tests use paired positions from the official Stockfish `UHO_Lichess_4852_v1.epd` suite:

```sh
mkdir -p matches/books
curl -L --fail \
  --output matches/books/UHO_Lichess_4852_v1.epd.zip \
  https://github.com/official-stockfish/books/raw/refs/heads/master/UHO_Lichess_4852_v1.epd.zip
unzip -o matches/books/UHO_Lichess_4852_v1.epd.zip -d matches/books
```

Validate the extracted file against the SHA-384 SRI value published in the official Stockfish
`books.json` manifest:

```sh
openssl dgst -sha384 -binary matches/books/UHO_Lichess_4852_v1.epd | openssl base64 -A
```

Expected value:

```text
QHAU1P3LurcJr7UTRI7HZCVFsoYBWC3OTsBqZY/FfQA6VQo3MmECWtByB4gVACW5
```

## Paired comparison

Always use `-repeat`: every opening must be played twice with colors reversed. Keep the engine
binary, hardware, time control, opening policy, seed, thread count, and adjudication rules fixed
throughout a comparison.

The next recommended bracket is Stockfish at `UCI_Elo=1500`, which is stronger than the previous
1320 anchor and close to Shrike's provisional point estimate. Start with 50 pairs (100 games):

```sh
/Users/yaarek/projects/tools/fastchess/fastchess \
  -engine \
    cmd=/Users/yaarek/projects/chess/matches/bin/shrike-current \
    name=Shrike-current \
  -engine \
    cmd=/opt/homebrew/bin/stockfish \
    name=Stockfish-1500 \
    option.UCI_LimitStrength=true \
    option.UCI_Elo=1500 \
    option.Threads=1 \
    option.Hash=16 \
  -each proto=uci tc=10+0.1 timemargin=250 \
  -openings \
    file=/Users/yaarek/projects/chess/matches/books/UHO_Lichess_4852_v1.epd \
    format=epd order=random \
  -srand 20260928 \
  -rounds 50 \
  -repeat \
  -concurrency 1 \
  -maxmoves 200 \
  -pgnout file=/Users/yaarek/projects/chess/matches/sf1500.pgn append=false \
  -log file=/Users/yaarek/projects/chess/matches/sf1500.log engine=true append=false
```

Do not use Fastchess `-strict` with `UCI_LimitStrength`: weakened Stockfish deliberately may choose
a `bestmove` different from the first move of its reported principal variation, which Fastchess
warns about even though it is expected behavior.

Fastchess writes a `config.json` autosave/recovery file to the working directory during a match.
After the run, move it under `matches/` (for example `matches/<run>-config.json`) so the
repository root stays clean; it is useful provenance because it captures the full match
configuration and running score.

Before using a result, inspect the log for crashes, illegal moves, stalls, and time forfeits. A
clean score between roughly 25% and 75% means the anchor is useful. If Shrike scores above 75%, move
to a stronger anchor; below 25%, move to a weaker one. After locating a useful anchor, use at least
250 pairs and retain the PGN, log, command, revisions, checksums, and Fastchess confidence interval.

For a more transferable final result, repeat the comparison at a longer time control such as
`60+0.6`; Stockfish's `UCI_Elo` is only an approximate external rating anchor and depends on the
time control and testing environment.

## Parallelism

Games are independent, so Fastchess `-concurrency` scales throughput almost linearly. Each game
runs two single-threaded engines, loading roughly two hardware threads, so keep concurrency at
about half the thread count minus headroom for the OS and controller.

Validated default on the 12-thread M6 Mac mini: `-concurrency 4` (2026-09-28: 20 games in 2:18,
20/20 normal terminations, 0 forfeits, 40% vs the 38% serial baseline with overlapping Elo
intervals; see `matches/sf1500-c4-summary.md`). Use it in place of `-concurrency 1` in the
command above.

Rules:

- Concurrency is part of the environment: fixed throughout a comparison and recorded in the
  experiment record. Never mix results across concurrency levels in one decision.
- Earlier serial results (for example the SF1500 bracket) remain valid standalone ratings.
- Re-validate before raising concurrency or moving hosts: a 20-40 game run must show no
  Shrike-side forfeits, stalls, or crashes, no forfeit-rate spike over baseline, and no score
  drift beyond noise.

## Current starting point

Commit `2333fde` was tested for 20 games against Stockfish 19 configured at `UCI_Elo=1320` and
`10+0.1`. Shrike scored 14 wins, 5 losses, and 1 draw. Fastchess reported a relative estimate of
`+168.40 +/- 307.24 Elo`.

Anchoring that point estimate to 1320 gives approximately **1488 Elo**, not 1518:

```text
1320 + 168.40 = 1488.40
```

The corresponding reported interval is approximately 1181–1796. This is only a bracketing signal:
20 games are far too few, the interval is very wide, and one Stockfish time forfeit favored Shrike.
It does, however, justify trying Stockfish 1500 next.
