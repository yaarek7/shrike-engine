# Template: do not fill in place, copy to `YYYY-MM-DD-<slug>.md`

```yaml
---
date: YYYY-MM-DD
question: One sentence: what is being tested?
type: improvement | rating | speed
status: complete
baseline: <rev-or-label, e.g. 2333fde>
candidate: <rev-or-label, or "same" for rating runs>
time_control: "10+0.1"
games: 100
openings: UHO_Lichess_4852_v1.epd, random, seed 20260928, paired
result: "+12.0 +/- 18.4 Elo, 45W-40L-15D (Fastchess)"
decision: accepted | rejected | neutral | superseded
supersedes: []
followups: []
---
```

## Hypothesis

Why should this change help? What is the expected direction and size?

## Setup

- Baseline binary: revision, SHA-256, config.
- Candidate binary: revision, SHA-256, config (or same-build A/B mechanism).
- Match conditions: opponent, time control, openings, adjudication,
  concurrency, host. Paste the Fastchess command.
- Stopping rule, pre-registered: SPRT bounds or fixed game count.

## Result

- Score and Fastchess statistical output, verbatim where practical.
- Log health: crashes, illegal moves, stalls, time forfeits.
- Speed numbers, if the experiment is performance-sensitive.

## Decision and reasoning

Accept, reject, or re-test, and why. Name what would change the decision.

## Lessons for future experiments

What should the next agent try (or avoid) based on this outcome?
