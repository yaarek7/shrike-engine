//! Public integration tests for FEN position handling.

use std::str::FromStr;

use chess_engine::chess::{CastleSide, Color, PieceKind, Position, Square};

#[test]
fn public_api_exposes_complete_position_state() {
    let fen = "r3k2r/8/8/3pP3/8/8/8/R3K2R w KQkq d6 14 37";
    let position = Position::from_fen(fen).expect("valid FEN");

    assert_eq!(position.side_to_move(), Color::White);
    assert_eq!(
        position.en_passant(),
        Some(Square::from_str("d6").expect("valid square"))
    );
    assert_eq!(position.halfmove_clock(), 14);
    assert_eq!(position.fullmove_number(), 37);
    assert_eq!(position.bitboard(Color::White, PieceKind::Rook).count(), 2);
    assert!(position.allows_castling(Color::Black, CastleSide::QueenSide));
    assert_eq!(position.to_fen(), fen);
}

#[test]
fn position_implements_standard_text_traits() {
    let parsed: Position = Position::STARTING_FEN.parse().expect("valid starting FEN");

    assert_eq!(parsed, Position::default());
    assert_eq!(parsed.to_string(), Position::STARTING_FEN);
}
