# Shrike versioning proposal

A version answers "what code is this?". A rating answers "how strong was this
build under one specific measurement?". This proposal keeps those answers
separate, because conflating them makes versions unreproducible: the same
binary re-measured under a different anchor, time control, or sample size
would need a different "version".

## The three layers

### 1. Code version (SemVer)

- Lives in `Cargo.toml` (`0.1.0` today) and is announced over UCI as
  `id name Shrike Engine <version>`.
- Follows SemVer while `0.x`: bump patch for fixes with no strength impact,
  minor for search/eval features and behavior changes. `1.0` waits until the
  engine is feature-complete by its own definition, not until a rating target.
- Each evaluated version gets a git tag (`v0.1.0`). The tag marks code, never
  a measurement.

### 2. Build identity (revision + checksum)

- The precise identity of a tested artifact is the full git revision plus the
  SHA-256 of the frozen binary, per `TESTING.md`.
- Frozen binaries are named `matches/bin/shrike-<shortrev>` (for example
  `shrike-2333fde`). `shrike-current` is only a pointer for the evaluation in
  progress, never a retained identity.
- Dirty-tree builds are never retained: commit first, then freeze.

### 3. Strength rating (ledger entry, not a version)

- A rating is recorded as a measurement with full context: anchor engine and
  settings, time control, games, score, Fastchess Elo estimate with confidence
  interval, date, binary revision and checksum.
- Per-run details live in `matches/<run>-summary.md` (git-ignored local
  artifacts). Promoted results live in `docs/experiments/` records (committed,
  see `EXPERIMENTS.md`).
- A number becomes the version's headline rating only after at least 250
  pairs on a useful anchor (score between 25% and 75%), per `TESTING.md`.
  Anything smaller is a bracketing signal and must be labeled as such.
- Human-friendly label when all three layers are known:

```text
Shrike 0.1.0 (2333fde, ~1488 @ SF1500/10+0.1, n=100)
```

## Rejected: `v.01.<date>.<rating>`

- `01` is not SemVer and matches neither `Cargo.toml` nor the UCI announcement.
- The date is redundant with the git revision and ambiguous (authored? built?
  measured?).
- The rating without anchor, time control, sample size, and error is
  misleading, and it changes on re-measurement while the code stays identical.
- If a rating must ever travel inside a version string, SemVer build metadata
  (`0.2.0+elo1488`) is the only acceptable slot, because it does not affect
  precedence. Prefer the ledger instead.

## Current versions

| Code | Revision | Binary SHA-256 (short) | Headline rating | Status |
|---|---|---|---|---|
| 0.1.0 | `2333fde` | `d466735c…` | ~1415 @ SF1500/10+0.1, n=100 (provisional) | bracketed, needs 250-pair promotion |

## Release checklist

1. Working tree clean; quality gate green (`TESTING.md`).
2. Bump `Cargo.toml` if the change warrants it; engine announces the version.
3. Freeze `matches/bin/shrike-<shortrev>`; record revision and checksum.
4. UCI compliance passes on the frozen binary.
5. Strength evidence recorded: bracket run first, then promotion run at 250+
   pairs for a headline rating.
6. Experiment record added under `docs/experiments/`; tag the revision.
