use std::{error::Error, fmt};

use super::{
    CastleSide, Color, Move, MoveKind, Piece, PieceKind, Position, Square, Undo,
    attacks::{
        bishop_directions, is_square_attacked, king_offsets, knight_offsets, offset,
        rook_directions,
    },
};

const BACK_RANKS: u64 = 0xFF00_0000_0000_00FF;

/// A structural or chess-legality problem with a position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PositionError {
    /// A color has no king.
    MissingKing(Color),
    /// A color has more than one king.
    MultipleKings(Color),
    /// A pawn occupies the first or eighth rank.
    PawnOnBackRank,
    /// The kings occupy adjacent squares.
    AdjacentKings,
    /// The side that just moved is still in check, so the position is unreachable.
    SideNotToMoveInCheck,
    /// Castling rights name a king or rook that is not on its home square.
    InconsistentCastlingRights,
    /// The en-passant target is inconsistent with the side to move or pawn placement.
    InconsistentEnPassant,
}

impl fmt::Display for PositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKing(color) => write!(formatter, "{color:?} king is missing"),
            Self::MultipleKings(color) => write!(formatter, "multiple {color:?} kings"),
            Self::PawnOnBackRank => formatter.write_str("a pawn occupies a back rank"),
            Self::AdjacentKings => formatter.write_str("the kings are adjacent"),
            Self::SideNotToMoveInCheck => formatter.write_str("the side not to move is in check"),
            Self::InconsistentCastlingRights => {
                formatter.write_str("castling rights do not match king and rook placement")
            }
            Self::InconsistentEnPassant => {
                formatter.write_str("en-passant target is inconsistent with the position")
            }
        }
    }
}

impl Error for PositionError {}

/// An error produced when attempting to make a move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveError {
    /// The starting position is invalid.
    InvalidPosition(PositionError),
    /// The move is not legal in the position.
    IllegalMove(Move),
}

impl fmt::Display for MoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPosition(error) => write!(formatter, "invalid position: {error}"),
            Self::IllegalMove(chess_move) => write!(formatter, "illegal move {chess_move}"),
        }
    }
}

impl Error for MoveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPosition(error) => Some(error),
            Self::IllegalMove(_) => None,
        }
    }
}

impl From<PositionError> for MoveError {
    fn from(error: PositionError) -> Self {
        Self::InvalidPosition(error)
    }
}

impl Position {
    /// Verifies the position invariants required by legal move generation.
    ///
    /// # Errors
    ///
    /// Returns [`PositionError`] if king, pawn, castling, en-passant, or check
    /// state is structurally inconsistent with a reachable orthodox position.
    pub fn validate(&self) -> Result<(), PositionError> {
        validate_position(self)
    }

    /// Returns whether `color` is currently in check.
    ///
    /// # Errors
    ///
    /// Returns [`PositionError`] if the position is invalid.
    pub fn is_in_check(&self, color: Color) -> Result<bool, PositionError> {
        validate_position(self)?;
        Ok(is_in_check_unchecked(self, color))
    }

    /// Generates every legal move for the side to move.
    ///
    /// # Errors
    ///
    /// Returns [`PositionError`] if the position is invalid.
    pub fn legal_moves(&self) -> Result<Vec<Move>, PositionError> {
        validate_position(self)?;
        Ok(generate_legal_moves_unchecked(self))
    }

    /// Makes a legal move and returns the state required to undo it.
    ///
    /// # Errors
    ///
    /// Returns [`MoveError::InvalidPosition`] when the current position is
    /// invalid, or [`MoveError::IllegalMove`] when the move is not legal.
    pub fn make_move(&mut self, chess_move: Move) -> Result<Undo, MoveError> {
        let legal = self.legal_moves()?;
        if !legal.contains(&chess_move) {
            return Err(MoveError::IllegalMove(chess_move));
        }
        Ok(self.apply_move_unchecked(chess_move))
    }
}

pub(crate) fn generate_legal_moves_unchecked(position: &Position) -> Vec<Move> {
    let moving_color = position.side_to_move();
    let mut working = position.clone();
    generate_pseudo_legal_moves(position)
        .into_iter()
        .filter(|&chess_move| {
            let undo = working.apply_move_unchecked(chess_move);
            let legal = !is_in_check_unchecked(&working, moving_color);
            working.unmake_move(undo);
            legal
        })
        .collect()
}

fn validate_position(position: &Position) -> Result<(), PositionError> {
    for color in [Color::White, Color::Black] {
        match position.bitboard(color, PieceKind::King).count() {
            0 => return Err(PositionError::MissingKing(color)),
            1 => {}
            _ => return Err(PositionError::MultipleKings(color)),
        }
    }

    let pawns = position.bitboard(Color::White, PieceKind::Pawn).bits()
        | position.bitboard(Color::Black, PieceKind::Pawn).bits();
    if pawns & BACK_RANKS != 0 {
        return Err(PositionError::PawnOnBackRank);
    }

    let white_king = king_square(position, Color::White);
    let black_king = king_square(position, Color::Black);
    if king_offsets()
        .iter()
        .any(|&(df, dr)| offset(white_king, df, dr) == Some(black_king))
    {
        return Err(PositionError::AdjacentKings);
    }

    validate_castling_rights(position)?;
    validate_en_passant(position)?;

    let side_not_to_move = position.side_to_move().opposite();
    if is_square_attacked(
        position,
        king_square(position, side_not_to_move),
        position.side_to_move(),
    ) {
        return Err(PositionError::SideNotToMoveInCheck);
    }
    Ok(())
}

fn validate_castling_rights(position: &Position) -> Result<(), PositionError> {
    for color in [Color::White, Color::Black] {
        let rank = home_rank(color);
        let king_home = Square::new(4, rank).expect("king home square is on board");
        let king = Some(Piece::new(color, PieceKind::King));
        for (side, rook_file) in [(CastleSide::QueenSide, 0), (CastleSide::KingSide, 7)] {
            if position.allows_castling(color, side) {
                let rook_home = Square::new(rook_file, rank).expect("rook home square is on board");
                if position.piece_at(king_home) != king
                    || position.piece_at(rook_home) != Some(Piece::new(color, PieceKind::Rook))
                {
                    return Err(PositionError::InconsistentCastlingRights);
                }
            }
        }
    }
    Ok(())
}

fn validate_en_passant(position: &Position) -> Result<(), PositionError> {
    let Some(target) = position.en_passant() else {
        return Ok(());
    };
    let (expected_rank, pawn_rank, origin_rank) = match position.side_to_move() {
        Color::White => (5, 4, 6),
        Color::Black => (2, 3, 1),
    };
    let pawn_square = Square::new(target.file(), pawn_rank).expect("pawn square is on board");
    let origin_square = Square::new(target.file(), origin_rank).expect("origin is on board");
    let expected_pawn = Piece::new(position.side_to_move().opposite(), PieceKind::Pawn);
    if target.rank() != expected_rank
        || position.piece_at(target).is_some()
        || position.piece_at(pawn_square) != Some(expected_pawn)
        || position.piece_at(origin_square).is_some()
        || position.halfmove_clock() != 0
    {
        return Err(PositionError::InconsistentEnPassant);
    }
    Ok(())
}

fn generate_pseudo_legal_moves(position: &Position) -> Vec<Move> {
    let color = position.side_to_move();
    let mut moves = Vec::with_capacity(64);
    generate_pawns(position, color, &mut moves);
    generate_leapers(
        position,
        color,
        PieceKind::Knight,
        knight_offsets(),
        &mut moves,
    );
    generate_sliders(
        position,
        color,
        PieceKind::Bishop,
        bishop_directions(),
        &mut moves,
    );
    generate_sliders(
        position,
        color,
        PieceKind::Rook,
        rook_directions(),
        &mut moves,
    );
    generate_sliders(
        position,
        color,
        PieceKind::Queen,
        bishop_directions(),
        &mut moves,
    );
    generate_sliders(
        position,
        color,
        PieceKind::Queen,
        rook_directions(),
        &mut moves,
    );
    generate_leapers(position, color, PieceKind::King, king_offsets(), &mut moves);
    generate_castling(position, color, &mut moves);
    moves
}

fn generate_pawns(position: &Position, color: Color, moves: &mut Vec<Move>) {
    let rank_delta = match color {
        Color::White => 1,
        Color::Black => -1,
    };
    let start_rank = match color {
        Color::White => 1,
        Color::Black => 6,
    };
    let promotion_rank = match color {
        Color::White => 7,
        Color::Black => 0,
    };

    for from in position.bitboard(color, PieceKind::Pawn).squares() {
        if let Some(to) = offset(from, 0, rank_delta) {
            if position.piece_at(to).is_none() {
                add_pawn_move(from, to, false, promotion_rank, moves);
                if from.rank() == start_rank {
                    if let Some(double_to) = offset(from, 0, rank_delta * 2) {
                        if position.piece_at(double_to).is_none() {
                            push_move(from, double_to, MoveKind::DoublePawnPush, moves);
                        }
                    }
                }
            }
        }

        for file_delta in [-1, 1] {
            let Some(to) = offset(from, file_delta, rank_delta) else {
                continue;
            };
            if position.en_passant() == Some(to) {
                push_move(from, to, MoveKind::EnPassant, moves);
            } else if let Some(target) = position.piece_at(to) {
                if target.color != color && target.kind != PieceKind::King {
                    add_pawn_move(from, to, true, promotion_rank, moves);
                }
            }
        }
    }
}

fn add_pawn_move(
    from: Square,
    to: Square,
    capture: bool,
    promotion_rank: u8,
    moves: &mut Vec<Move>,
) {
    if to.rank() == promotion_rank {
        let kinds = if capture {
            [
                MoveKind::KnightPromotionCapture,
                MoveKind::BishopPromotionCapture,
                MoveKind::RookPromotionCapture,
                MoveKind::QueenPromotionCapture,
            ]
        } else {
            [
                MoveKind::KnightPromotion,
                MoveKind::BishopPromotion,
                MoveKind::RookPromotion,
                MoveKind::QueenPromotion,
            ]
        };
        for kind in kinds {
            push_move(from, to, kind, moves);
        }
    } else {
        push_move(
            from,
            to,
            if capture {
                MoveKind::Capture
            } else {
                MoveKind::Quiet
            },
            moves,
        );
    }
}

fn generate_leapers(
    position: &Position,
    color: Color,
    kind: PieceKind,
    offsets: &[(i8, i8)],
    moves: &mut Vec<Move>,
) {
    for from in position.bitboard(color, kind).squares() {
        for &(df, dr) in offsets {
            let Some(to) = offset(from, df, dr) else {
                continue;
            };
            match position.piece_at(to) {
                None => push_move(from, to, MoveKind::Quiet, moves),
                Some(target) if target.color != color && target.kind != PieceKind::King => {
                    push_move(from, to, MoveKind::Capture, moves);
                }
                Some(_) => {}
            }
        }
    }
}

fn generate_sliders(
    position: &Position,
    color: Color,
    kind: PieceKind,
    directions: &[(i8, i8)],
    moves: &mut Vec<Move>,
) {
    for from in position.bitboard(color, kind).squares() {
        for &(df, dr) in directions {
            let mut current = from;
            while let Some(to) = offset(current, df, dr) {
                match position.piece_at(to) {
                    None => push_move(from, to, MoveKind::Quiet, moves),
                    Some(target) if target.color != color && target.kind != PieceKind::King => {
                        push_move(from, to, MoveKind::Capture, moves);
                        break;
                    }
                    Some(_) => break,
                }
                current = to;
            }
        }
    }
}

fn generate_castling(position: &Position, color: Color, moves: &mut Vec<Move>) {
    let rank = home_rank(color);
    let king_from = Square::new(4, rank).expect("king home square is on board");
    if is_square_attacked(position, king_from, color.opposite()) {
        return;
    }
    for (side, empty_files, transit_files, kind, king_to_file) in [
        (
            CastleSide::KingSide,
            &[5_u8, 6_u8][..],
            &[5_u8, 6_u8][..],
            MoveKind::KingCastle,
            6,
        ),
        (
            CastleSide::QueenSide,
            &[1_u8, 2_u8, 3_u8][..],
            &[3_u8, 2_u8][..],
            MoveKind::QueenCastle,
            2,
        ),
    ] {
        if !position.allows_castling(color, side)
            || !empty_files.iter().all(|&file| {
                position
                    .piece_at(Square::new(file, rank).expect("castling path is on board"))
                    .is_none()
            })
            || !transit_files.iter().all(|&file| {
                !is_square_attacked(
                    position,
                    Square::new(file, rank).expect("castling path is on board"),
                    color.opposite(),
                )
            })
        {
            continue;
        }
        let king_to = Square::new(king_to_file, rank).expect("castling destination is on board");
        push_move(king_from, king_to, kind, moves);
    }
}

fn push_move(from: Square, to: Square, kind: MoveKind, moves: &mut Vec<Move>) {
    moves.push(Move::new(from, to, kind).expect("generated move has distinct squares"));
}

fn king_square(position: &Position, color: Color) -> Square {
    position
        .bitboard(color, PieceKind::King)
        .squares()
        .next()
        .expect("validated position has exactly one king")
}

fn is_in_check_unchecked(position: &Position, color: Color) -> bool {
    is_square_attacked(position, king_square(position, color), color.opposite())
}

const fn home_rank(color: Color) -> u8 {
    match color {
        Color::White => 0,
        Color::Black => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::{MoveError, PositionError};
    use crate::chess::{Move, MoveKind, Position, Square, zobrist};

    fn move_by_uci(position: &Position, uci: &str) -> Move {
        position
            .legal_moves()
            .expect("test position is valid")
            .into_iter()
            .find(|chess_move| chess_move.to_uci() == uci)
            .expect("requested move is legal")
    }

    #[test]
    fn starting_position_has_twenty_legal_moves() {
        let moves = Position::starting()
            .legal_moves()
            .expect("startpos is valid");

        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn make_and_unmake_restore_exact_position() {
        let mut position = Position::starting();
        let original = position.clone();
        let chess_move = move_by_uci(&position, "e2e4");
        let undo = position.make_move(chess_move).expect("E2-E4 is legal");

        assert_eq!(
            position.to_fen(),
            "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
        );
        position.unmake_move(undo);
        assert_eq!(position, original);
    }

    #[test]
    fn incremental_repetition_keys_match_full_recomputation() {
        for fen in [
            Position::STARTING_FEN,
            "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
            "7k/8/8/3pP3/4K3/8/8/8 w - d6 0 1",
            "r3k3/1P6/8/8/8/8/8/4K3 w - - 0 1",
        ] {
            let position = Position::from_fen(fen).expect("valid FEN");
            for chess_move in position.legal_moves().expect("valid position") {
                let mut child = position.clone();
                let original_key = child.repetition_key();
                let undo = child
                    .make_move(chess_move)
                    .expect("generated move is legal");

                assert_eq!(
                    child.repetition_key(),
                    zobrist::recompute(&child),
                    "incremental key mismatch after {chess_move} from {fen}"
                );
                child.unmake_move(undo);
                assert_eq!(child.repetition_key(), original_key);
                assert_eq!(child, position);
            }
        }
    }

    #[test]
    fn illegal_move_is_rejected_without_mutation() {
        let mut position = Position::starting();
        let original = position.clone();
        let e2: Square = "e2".parse().expect("valid square");
        let e5: Square = "e5".parse().expect("valid square");
        let illegal = Move::new(e2, e5, MoveKind::Quiet).expect("different squares");

        assert_eq!(
            position.make_move(illegal),
            Err(MoveError::IllegalMove(illegal))
        );
        assert_eq!(position, original);
    }

    #[test]
    fn syntax_valid_position_without_kings_fails_validation() {
        let position = Position::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").expect("valid FEN");

        assert_eq!(
            position.validate(),
            Err(PositionError::MissingKing(crate::chess::Color::White))
        );
    }

    #[test]
    fn en_passant_that_exposes_own_king_is_illegal() {
        let position = Position::from_fen("8/8/8/r4pPK/8/8/8/4k3 w - f6 0 1").expect("valid FEN");
        let moves = position.legal_moves().expect("position is valid");

        assert!(!moves.iter().any(|chess_move| chess_move.to_uci() == "g5f6"));
    }

    #[test]
    fn castling_through_check_is_illegal() {
        let position = Position::from_fen("k4r2/8/8/8/8/8/8/4K2R w K - 0 1").expect("valid FEN");
        let moves = position.legal_moves().expect("position is valid");

        assert!(!moves.iter().any(|chess_move| chess_move.to_uci() == "e1g1"));
    }

    #[test]
    fn en_passant_can_remove_a_checking_pawn() {
        let position = Position::from_fen("7k/8/8/3pP3/4K3/8/8/8 w - d6 0 1").expect("valid FEN");
        let moves = position.legal_moves().expect("position is valid");

        assert!(moves.iter().any(|chess_move| chess_move.to_uci() == "e5d6"));
    }

    #[test]
    fn double_check_allows_only_king_moves() {
        let position = Position::from_fen("k3r3/8/8/8/1b6/8/8/4K3 w - - 0 1").expect("valid FEN");
        let moves = position.legal_moves().expect("position is valid");
        let king_square: Square = "e1".parse().expect("valid square");

        assert!(!moves.is_empty());
        assert!(
            moves
                .iter()
                .all(|chess_move| chess_move.from() == king_square)
        );
    }

    #[test]
    fn every_special_move_unmakes_exactly() {
        for (fen, uci) in [
            ("4k3/8/8/8/8/8/8/4K2R w K - 0 1", "e1g1"),
            ("7k/8/8/3pP3/4K3/8/8/8 w - d6 0 1", "e5d6"),
            ("r3k3/1P6/8/8/8/8/8/4K3 w - - 0 1", "b7a8q"),
        ] {
            let mut position = Position::from_fen(fen).expect("valid FEN");
            let original = position.clone();
            let chess_move = move_by_uci(&position, uci);
            let undo = position
                .make_move(chess_move)
                .expect("fixture move is legal");
            position.unmake_move(undo);
            assert_eq!(position, original, "failed to restore after {uci}");
        }
    }

    #[test]
    fn rook_move_and_home_square_capture_revoke_rights() {
        let mut position =
            Position::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 5 10").expect("valid FEN");
        let rook_capture = move_by_uci(&position, "a1a8");
        position
            .make_move(rook_capture)
            .expect("rook capture is legal");

        assert!(!position.allows_castling(
            crate::chess::Color::White,
            crate::chess::CastleSide::QueenSide
        ));
        assert!(position.allows_castling(
            crate::chess::Color::White,
            crate::chess::CastleSide::KingSide
        ));
        assert!(!position.allows_castling(
            crate::chess::Color::Black,
            crate::chess::CastleSide::QueenSide
        ));
        assert!(position.allows_castling(
            crate::chess::Color::Black,
            crate::chess::CastleSide::KingSide
        ));
        assert_eq!(position.halfmove_clock(), 0);
        assert_eq!(position.fullmove_number(), 10);
    }

    #[test]
    fn quiet_moves_increment_clocks_and_black_advances_fullmove() {
        let mut position =
            Position::from_fen("4k3/8/8/8/8/8/8/R3K3 w - - 7 10").expect("valid FEN");
        let original = position.clone();

        let white_move = move_by_uci(&position, "a1a2");
        let white_undo = position.make_move(white_move).expect("rook move is legal");
        assert_eq!(position.halfmove_clock(), 8);
        assert_eq!(position.fullmove_number(), 10);

        let black_move = move_by_uci(&position, "e8e7");
        let black_undo = position.make_move(black_move).expect("king move is legal");
        assert_eq!(position.halfmove_clock(), 9);
        assert_eq!(position.fullmove_number(), 11);

        position.unmake_move(black_undo);
        position.unmake_move(white_undo);
        assert_eq!(position, original);
    }

    #[test]
    fn all_four_promotions_are_generated() {
        let position = Position::from_fen("4k3/P7/8/8/8/8/8/4K3 w - - 0 1").expect("valid FEN");
        let mut promotions: Vec<_> = position
            .legal_moves()
            .expect("position is valid")
            .into_iter()
            .filter(|chess_move| chess_move.from().to_string() == "a7")
            .map(Move::to_uci)
            .collect();
        promotions.sort();

        assert_eq!(promotions, ["a7a8b", "a7a8n", "a7a8q", "a7a8r"]);
    }
}
