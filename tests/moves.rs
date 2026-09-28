//! Public integration tests for compact move values.

use std::mem::size_of;

use chess_engine::chess::{Move, MoveKind, PieceKind, Square};

#[test]
fn public_move_api_preserves_semantics_in_two_bytes() {
    let from: Square = "e7".parse().expect("valid source square");
    let to: Square = "f8".parse().expect("valid destination square");
    let chess_move = Move::new(from, to, MoveKind::QueenPromotionCapture)
        .expect("different squares form a move");

    assert_eq!(size_of::<Move>(), 2);
    assert_eq!(chess_move.from(), from);
    assert_eq!(chess_move.to(), to);
    assert_eq!(chess_move.kind(), MoveKind::QueenPromotionCapture);
    assert!(chess_move.is_capture());
    assert_eq!(chess_move.promotion(), Some(PieceKind::Queen));
    assert_eq!(chess_move.to_uci(), "e7f8q");
    assert_eq!(Move::from_raw(chess_move.raw()), Some(chess_move));
}
