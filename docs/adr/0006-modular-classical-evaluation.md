# ADR-0006: Modular classical evaluation

- Status: Accepted
- Date: 2026-09-27

## Context

The first search needs a deterministic leaf evaluator, but future milestones must be able to
replace it with NNUE or experimental learned evaluators. Evaluation changes also need to remain
measurable independently rather than accumulating in an opaque formula.

## Decision

Scores use a distinct, bounded centipawn type. Evaluators implement a small trait and return a
score from the side-to-move perspective. The classical evaluator separately exposes material,
activity, pseudo-mobility, pawn-structure, and king-safety components. Components and totals use
White's perspective; callers can request either color's perspective through a sign conversion.

The initial evaluator is deliberately simple: conventional material values, centralization and
pawn advancement, pseudo-legal mobility, doubled/isolated/passed pawn terms, and king pawn-shield
and castled-position bonuses. No evaluation term changes chess state or move legality.

## Consequences

- Search can depend on the evaluator trait rather than a particular formula.
- Tests and future tuning can identify which term caused a score change.
- Pseudo-mobility intentionally ignores pins and attacked king destinations; it is a heuristic,
  not a second legal move generator.
- Parameters are explicit code constants for the baseline and will require controlled experiments
  before tuning infrastructure is introduced.

## Verification

- The starting position is exactly balanced in every component.
- Material values are checked in centipawns.
- Color-perspective scores are antisymmetric.
- Centralization, mobility, pawn structure, and king shield behavior have focused tests.
- An integration test verifies that component scores sum exactly to the total.
