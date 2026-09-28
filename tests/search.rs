//! Public integration tests for fixed-depth search.

use chess_engine::{
    chess::{Move, Position},
    eval::ClassicalEvaluator,
    search::{search, search_with_history},
};

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
