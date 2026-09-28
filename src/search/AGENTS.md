# Search extension map

Use the highest-level seam that fits the experiment:

1. Evaluation-only variant: implement `Evaluator`, then construct `AlphaBetaSearcher<E>`.
2. Alpha-beta policy variant: add a narrow option to `AlphaBetaConfig` and keep its branch outside
   repeated work where possible.
3. Different methodology: implement `SearchBackend`; inject it with `uci::run_with_backend`.
4. Protocol-only behavior: change `src/uci/mod.rs`, not node search.

Stable public contracts are `SearchBackend`, `AlphaBetaSearcher`, `AlphaBetaConfig`,
`SearchLimits`, `StopToken`, `SearchResult`, `IterationInfo`, and `IterativeSearchResult`.
Compatibility free functions use the default alpha-beta configuration.

The private `SearchContext` owns hot-path state. Avoid trait-object calls inside `negamax` or
`quiescence`; composition happens at the root, and policy enums are statically matched.

Use targeted symbol search instead of reading the whole module: `rg -n "SearchBackend|run_iteration|\
fn negamax|fn quiescence|mod tests" src/search/mod.rs`.

For same-build A/B tests, instantiate two backends from the same position/history and compare
score, legal move/PV, completed depth, and deterministic node counts. Strength claims still require
controlled matches; node reductions alone are performance evidence, not playing-strength evidence.
