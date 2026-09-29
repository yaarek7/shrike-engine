# Working map for coding agents

Read only the files routed by the change. Do not load all ADRs or the full test suite up front.

## Change routing

- Board state, FEN, hashing, legality, make/unmake: `src/chess/` and `tests/perft.rs`.
- Evaluation terms only: `src/eval/` and `tests/evaluation.rs`.
- Search algorithms, policies, limits, results: `src/search/mod.rs` and
  `src/search/AGENTS.md`.
- UCI parsing, clocks, worker lifecycle, protocol output: `src/uci/mod.rs` and `tests/uci.rs`.
- Stable architecture decisions: newest relevant file under `docs/adr/`; do not read unrelated ADRs.
- Strength experiments and ratings: `EXPERIMENTS.md`, `VERSIONING.md`, and records under
  `docs/experiments/`; match workflow in `TESTING.md`.

## Required discipline

- Preserve the public compatibility helpers unless the task explicitly changes the API.
- Compare search experiments through `SearchBackend`/`AlphaBetaSearcher`; do not hardcode a new
  methodology into UCI.
- Keep evaluator and search changes in separate experiments.
- Any chess-state or move-generation change requires release perft. Search-only work does not need
  the ignored deep perft suite unless it changes position mutation.
- Prefer deterministic node assertions to timing assertions. Process timing tests need broad bounds.

## Smallest useful checks

- Evaluation: `cargo test --lib eval --locked && cargo test --test evaluation --locked`
- Search: `cargo test --lib search --locked && cargo test --test search --locked`
- UCI/controller: `cargo test --lib uci --locked && cargo test --test uci --locked`
- Chess rules: `cargo test --lib chess --locked && cargo test --release --test perft --locked`
- Before commit: run the complete gate documented in `docs/DEVELOPMENT.md`.
