//! Public integration tests for classical evaluation.

use chess_engine::{
    chess::{Color, Position},
    eval::{ClassicalEvaluator, Evaluator},
};

#[test]
fn evaluation_components_sum_to_the_reported_total() {
    let position = Position::from_fen(
        "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/2N2N2/PPPP1PPP/R1BQK2R w KQkq - 4 5",
    )
    .expect("valid FEN");
    let evaluation = ClassicalEvaluator.breakdown(&position);
    let component_sum = evaluation.material().centipawns()
        + evaluation.activity().centipawns()
        + evaluation.mobility().centipawns()
        + evaluation.pawn_structure().centipawns()
        + evaluation.king_safety().centipawns();

    assert_eq!(evaluation.total().centipawns(), component_sum);
    assert_eq!(
        ClassicalEvaluator.evaluate_for(&position, Color::Black),
        -evaluation.total()
    );
}
