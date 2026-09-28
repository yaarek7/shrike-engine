//! Public integration tests for search composition and fixed-depth search.

use std::time::Duration;

use chess_engine::{
    chess::{Move, Position, RepetitionKey},
    eval::{ClassicalEvaluator, Score},
    search::{
        IterationInfo, IterativeSearchResult, SearchBackend, SearchError, SearchLimits,
        SearchResult, SearchTermination, StopToken, search, search_with_history,
    },
};

struct FirstLegalMoveBackend;

impl SearchBackend for FirstLegalMoveBackend {
    fn fixed_search(
        &self,
        position: &Position,
        _prior_position_keys: &[RepetitionKey],
        depth: u8,
    ) -> Result<SearchResult, SearchError> {
        if !(1..=64).contains(&depth) {
            return Err(SearchError::InvalidDepth(depth));
        }

        let best_move = position.legal_moves()?.into_iter().next();
        let principal_variation = best_move.into_iter().collect();
        Ok(SearchResult::new(
            depth,
            Score::ZERO,
            1,
            0,
            best_move,
            principal_variation,
        ))
    }

    fn iterative_search(
        &self,
        position: &Position,
        prior_position_keys: &[RepetitionKey],
        limits: SearchLimits,
        _stop: &StopToken,
        on_iteration: &mut dyn FnMut(&IterationInfo),
    ) -> Result<IterativeSearchResult, SearchError> {
        let result = self.fixed_search(position, prior_position_keys, limits.depth)?;
        on_iteration(&IterationInfo::new(result.clone(), 1, Duration::ZERO));

        Ok(IterativeSearchResult::new(
            Some(result.clone()),
            result.best_move(),
            1,
            Duration::ZERO,
            SearchTermination::Completed,
        ))
    }
}

#[test]
fn public_search_backend_can_be_implemented_and_injected_externally() {
    let backend: &dyn SearchBackend = &FirstLegalMoveBackend;
    let position = Position::starting();
    let mut reported_depth = None;
    let result = backend
        .iterative_search(
            &position,
            &[],
            SearchLimits::depth(2),
            &StopToken::default(),
            &mut |iteration| reported_depth = Some(iteration.result().depth()),
        )
        .expect("custom backend search succeeds");

    assert_eq!(reported_depth, Some(2));
    assert_eq!(result.completed().map(SearchResult::depth), Some(2));
    assert!(result.best_move().is_some());
    assert_eq!(result.termination(), SearchTermination::Completed);
}

#[test]
fn public_search_result_is_legal_replayable_and_deterministic() {
    let position = Position::from_fen(
        "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 4 5",
    )
    .expect("valid FEN");
    let first = search(&position, &ClassicalEvaluator, 3).expect("search succeeds");
    let second = search(&position, &ClassicalEvaluator, 3).expect("search succeeds");

    assert_eq!(first, second);
    assert!(first.nodes() > 1);
    assert!(first.quiescence_nodes() > 0);
    assert!(first.nodes() >= first.quiescence_nodes());
    assert!(first.principal_variation().len() <= 3);
    assert_eq!(
        first.best_move(),
        first.principal_variation().first().copied()
    );

    let mut replay = position;
    for &chess_move in first.principal_variation() {
        replay.make_move(chess_move).expect("PV move is legal");
    }
}

#[test]
fn public_search_uses_quiescence_without_extending_the_reported_pv() {
    let position = Position::from_fen("4k3/8/5n2/3p4/8/8/8/3QK3 w - - 0 1").expect("valid FEN");
    let result = search(&position, &ClassicalEvaluator, 1).expect("search succeeds");

    assert_ne!(
        result.best_move().map(Move::to_uci).as_deref(),
        Some("d1d5")
    );
    assert_eq!(result.principal_variation().len(), 1);
    assert!(result.quiescence_nodes() > 0);
}

#[test]
fn public_history_aware_search_adjudicates_repetition() {
    let position = Position::from_fen("4k3/8/8/8/8/8/7Q/4K3 b - - 0 1").expect("valid FEN");
    let prior = [position.repetition_key(), position.repetition_key()];
    let result =
        search_with_history(&position, &ClassicalEvaluator, 2, &prior).expect("search succeeds");

    assert_eq!(result.score().centipawns(), 0);
    assert!(result.nodes() > 1);
    assert!(result.best_move().is_some());
}

#[test]
fn tactical_capture_and_mate_are_selected() {
    for (fen, depth, expected) in [
        ("4k3/8/8/8/8/8/q7/R3K3 w - - 0 1", 2, "a1a2"),
        ("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1", 2, "f7f8"),
    ] {
        let position = Position::from_fen(fen).expect("valid FEN");
        let result = search(&position, &ClassicalEvaluator, depth).expect("search succeeds");

        assert_eq!(
            result.best_move().map(Move::to_uci).as_deref(),
            Some(expected)
        );
    }
}
