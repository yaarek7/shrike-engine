# ADR-0011: Root-level search composition boundary

- Status: Accepted
- Date: 2026-09-28

## Context

Chess rules, evaluation, search, and UCI were separate modules, but the executable still selected
the classical evaluator and one concrete search function. Negamax, quiescence, and ordering were
hardwired inside `SearchContext`. Controlled comparisons therefore required editing code or
building separate revisions, despite the architecture's replaceability goal.

The extension seam must make same-build comparisons straightforward without adding trait-object
calls at every node. It should also preserve the existing public functions so previous tests and
external callers do not require a migration.

## Decision

`SearchBackend` is the object-safe root interface for exact and controlled iterative searches.
UCI exposes `run_with_backend` and receives an `Arc<dyn SearchBackend>`; its normal `run` composes
the production default. Dynamic dispatch therefore occurs at the search boundary, not recursively.

`AlphaBetaSearcher<E>` is a statically dispatched backend that owns any thread-safe `Evaluator`
and an `AlphaBetaConfig`. The initial independent policies are `QuiescencePolicy` and
`MoveOrderingPolicy`. Default values retain the accepted M8 behavior. Disabling quiescence gives a
deliberate horizon-search baseline, while encoded-only ordering provides a deterministic ordering
baseline. Existing free search functions delegate to the default configuration.

New methodologies implement `SearchBackend`; alpha-beta experiments add narrowly scoped policy
values rather than protocol branches. Same-build tests instantiate both baseline and candidate
against identical position, history, and limits.

Root and search-specific `AGENTS.md` files provide change routing, stable seams, and the smallest
relevant verification commands. They are intentionally concise and direct future agents toward
symbols instead of whole-file reads.

## Consequences

- UCI no longer determines search methodology and can exercise injected backends.
- Evaluator, quiescence, and ordering variants can be compared in one executable or test build.
- Node recursion retains static evaluator dispatch and enum policy branches rather than virtual
  calls.
- `SearchBackend` intentionally describes root behavior. Fine-grained node policies remain alpha-
  beta configuration, avoiding an excessively abstract hot path.
- The search implementation remains in one source module for now; concise agent routing reduces
  discovery cost without a large mechanical file move. It can be split later when ownership
  boundaries become large enough to justify the churn.

## Verification

- A same-build poisoned-capture test compares quiescence enabled and disabled and observes the
  expected different move plus quiescence-node count.
- Tactical and encoded-only ordering return the same exact score.
- UCI accepts an injected backend with a non-default evaluator and reports its score.
- A public integration test implements a backend using only exported composition APIs.
- All legacy free-function, controller, draw, UCI, and process tests remain unchanged in behavior.
