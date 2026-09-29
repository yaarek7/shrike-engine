# Experiment records

Each file here is one completed experiment: an improvement attempt or a
strength evaluation, accepted or rejected. Agents proposing engine changes
must read the index below and any record touching the same subsystem before
designing a new experiment, and must link follow-ups to the records they
supersede.

## Index

| Date | Record | Question | Decision |
|---|---|---|---|
| 2026-09-28 | [baseline-rating-sf1500](2026-09-28-baseline-rating-sf1500.md) | Strength of 0.1.0 (`2333fde`)? | neutral (bracket ~1415, needs promotion run) |

## How to add a record

1. Copy `TEMPLATE.md` to `YYYY-MM-DD-<slug>.md`.
2. Fill the frontmatter completely; it is the machine-readable summary agents
   scan first. Keep `question` to one sentence and `decision` to one of
   `accepted`, `rejected`, `neutral`, or `superseded`.
3. Write the narrative for a future agent: enough setup detail to replicate,
   the statistical result as reported (never rounded to a bare point
   estimate), and the reasoning behind the decision.
4. Link related records in `supersedes` / `followups` so dead ideas stay dead
   and live questions stay visible.
5. Add one row to the index above.

Raw PGNs, logs, and binaries stay in the git-ignored `matches/` directory;
records quote the command, revisions, checksums, and Fastchess result so the
conclusion survives without them.
