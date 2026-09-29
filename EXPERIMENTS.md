# Shrike experiment process

This guide turns "I have an idea to improve the engine" into an accepted or
rejected change with measured impact. It complements `TESTING.md` (how to run
a valid match) and `VERSIONING.md` (how versions and ratings relate).

## The two match types

| | Absolute (anchor) match | Relative (A/B) match |
|---|---|---|
| Question | How strong is this build? | Is the candidate better than the baseline? |
| Opponent | Weakened Stockfish at fixed `UCI_Elo` | Baseline Shrike vs candidate Shrike |
| Output | Ledger rating (`~1488 @ SF1500/10+0.1`) | Elo delta with confidence (`+12 ± 9`) |
| When | New version headline rating | Every proposed strength improvement |

Do not decide an improvement by comparing two anchor ratings: anchor noise
dwarfs small deltas. Decide improvements head-to-head, then re-anchor the
winner only when a new headline rating is needed.

## Lifecycle of an improvement experiment

### 1. Hypothesis

Write one sentence before touching code: what changes, and why it should gain
strength (or speed). Name the expected direction and rough size. If the idea
touches both search and evaluation, split it: one independent variable per
experiment (`docs/DEVELOPMENT.md`).

### 2. Design

- Baseline: the parent commit, frozen per `TESTING.md`. Candidate: baseline
  plus exactly one change.
- Prefer same-build A/B when the change fits behind the existing seams:
  `SearchBackend`/`AlphaBetaSearcher` with swappable evaluator, quiescence,
  and move-ordering policies. Same-build tests remove build and environment
  noise. Otherwise freeze two binaries and test them as separate engines.
- Pre-register the match conditions: time control, opening suite and policy,
  adjudication, thread count, and the stopping rule. Write them in the
  experiment record before the match starts.

### 3. Correctness gate

Run the `TESTING.md` repository checks on the candidate. Chess-state or
move-generation changes additionally require release perft. A candidate that
fails correctness is rejected without a match.

### 4. Speed check (when performance-sensitive)

If the change can affect speed, take reproducible before/after node counts or
timings on fixed positions first. Strength and speed are separate claims:
report both, decide on strength unless the experiment is explicitly about
speed. Prefer deterministic node assertions to timing assertions.

### 5. Strength match

- Always paired colors (`-repeat`), fixed binary, hardware, time control,
  openings, seed, threads, and adjudication throughout.
- Fast time control (for example `10+0.1`) for screening; confirm accepted
  improvements at a longer control (for example `60+0.6`) before release, since
  scaling behavior differs.
- Stopping rules, in order of preference:
  1. **SPRT** for accept/reject decisions (Fastchess `-sprt elo0=0 elo1=5
     alpha=0.05 beta=0.05 model=normalized`), which stops early with
     controlled error rates.
  2. **Fixed games** with a pre-registered count (screening: 100+ games;
     decision: 500+ games). Never stop a fixed-game test early because the
     score looks good; that invalidates the confidence interval.
- Read the result as Fastchess reports it: Elo delta, confidence interval,
  pentanomial stats, and LOS. A delta whose interval excludes zero at the
  chosen bounds is a signal; anything else is neutral, not "slightly better".
- Inspect logs for crashes, illegal moves, stalls, and time forfeits before
  believing any score.

### 6. Decision

- **Accept**: SPRT accepts H1, or the fixed-game interval excludes zero in the
  candidate's favor at both screening and (for release) longer controls.
- **Reject**: SPRT accepts H0, the interval includes zero or favors the
  baseline, or correctness/speed costs outweigh a marginal gain.
- **Re-test**: narrow neutral results may earn one pre-registered replication
  with more games or a longer control, not repeated peeking.

### 7. Record

Every experiment ends as a record under `docs/experiments/`, accepted or not.
Negative results are the most valuable records: they stop the next agent from
re-testing a dead idea. See `docs/experiments/README.md` for the format.

## Guidance for future experimenters

- Small deltas need big samples: ±10 Elo noise at 100 games hides most
  single-patch gains. Use SPRT or 500+ games for fine-grained decisions.
- Draw rate matters: high draws shrink Elo error bars but also shrink deltas;
  report the draw rate alongside every result.
- Time control is part of the claim: `+15 @ 10+0.1` may vanish at `60+0.6`.
  Never generalize across controls without measuring.
- Environment is part of the setup: same machine, same load, one concurrency
  unless a higher value was validated for timing stability.
- When in doubt, re-run the baseline rather than trusting a stale number.
