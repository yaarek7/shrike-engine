# 2026-09-28: Baseline rating bracket for 0.1.0 (`2333fde`)

```yaml
---
date: 2026-09-28
question: What is the playing strength of Shrike 0.1.0 (2333fde)?
type: rating
status: complete
baseline: 2333fde
candidate: same
time_control: "10+0.1"
games: 100
openings: UHO_Lichess_4852_v1.epd, random, seed 20260928, paired
result: "-85.04 +/- 60.51 Elo vs SF1500, 36W-60L-4D (95% pentanomial CI)"
decision: neutral
supersedes: []
followups: ["250+ pair promotion run at SF1500", "long-TC confirmation at 60+0.6"]
---
```

## Hypothesis

The 20-game smoke run against SF1320 implied ~1488 with a very wide interval
(`+168.40 +/- 307.24`). SF1500 should sit closer to Shrike's true strength and
produce a better-centered bracket at 100 games.

## Setup

- Binary: `2333fde`, SHA-256 `d466735c…`, frozen as `matches/bin/shrike-current`
  (identical checksum to the retained `shrike-2333fde`).
- Pre-run gates: `cargo fmt --check`, Clippy `-D warnings`, full test suite
  (99 lib + integration, 0 failures), `cargo doc -D warnings`, release build,
  Fastchess UCI compliance 40/40, opening-book SHA-384 verified.
- Opponent: Stockfish 19, `UCI_LimitStrength=true`, `UCI_Elo=1500`, 1 thread,
  16 MB hash. No `-strict` (expected PV/bestmove mismatch under
  `UCI_LimitStrength`).
- Full Fastchess command and conditions in `matches/sf1500-summary.md`
  (git-ignored local artifacts: `sf1500.pgn`, `sf1500.log`,
  `sf1500-config.json`).
- Stopping rule, pre-registered: fixed 50 rounds with `-repeat` (100 games).

## Result

- Score 38.0/100 (36W-60L-4D), inside the useful 25-75% band: the anchor is good.
- Pentanomial WW=4 WD=1 WL=27 DD=0 LD=3 LL=15 gives `-85.04 +/- 60.51`
  (95% CI), anchored to **~1415 (approx 1354-1473)**. The CI computation was
  validated by reproducing the smoke run's Fastchess output exactly.
- Log health: 99 normal terminations, 0 Shrike crashes, 0 illegal moves or
  disconnects. One Stockfish time forfeit (Shrike win, ply 28) mirrors the
  smoke run's forfeit pattern; it favors Shrike by ~5 Elo.
- Note the tension with the smoke-implied ~1488 @ SF1320: different anchors
  absolutize differently, as expected from an approximate `UCI_Elo` scale. The
  SF1500 bracket (5x the games, well-centered score) supersedes the smoke
  number as the provisional estimate.

## Decision and reasoning

Neutral (bracket, not headline): 100 games at one short time control do not
meet the 250-pair promotion bar. Provisional rating **~1415 @ SF1500/10+0.1,
n=100**. A promotion run and a `60+0.6` confirmation are the defined followups.

## Lessons for future experiments

- SF1500 at `10+0.1` is the current best anchor for builds near this strength;
  reuse it for comparability until Shrike drifts out of the 25-75% band.
- Weakened Stockfish forfeits on time occasionally (~1% here and in smoke);
  always attribute forfeits and report the score with and without them.
- Fastchess stdout summaries can be lost to pipes; the `config.json` autosave
  (moved to `matches/<run>-config.json`) preserves pentanomial stats for
  exact CI reconstruction.
- Early-match pace misleads: the first 20 games ran ~3x faster than the
  match average. Estimate ETAs from at least 50 games.
