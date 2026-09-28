//! Deterministic Zobrist keys for repetition identity.

use super::{CastleSide, Color, PieceKind, Position, Square, attacks::is_square_attacked};

const PIECE_KEYS: u64 = 12 * 64;
const SIDE_KEY: u64 = PIECE_KEYS;
const CASTLING_KEYS: u64 = SIDE_KEY + 1;
const EN_PASSANT_KEYS: u64 = CASTLING_KEYS + 4;
const SEED: u64 = 0x5348_5249_4B45_0001;

/// A deterministic key for the aspects of a position relevant to repetition.
///
/// Move clocks are intentionally excluded. An en-passant file contributes only
/// when the side to move has a legal en-passant capture, matching FIDE position
/// identity rather than raw FEN equality.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RepetitionKey(u64);

impl RepetitionKey {
    pub(super) const ZERO: Self = Self(0);

    /// Returns the underlying deterministic 64-bit value.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }

    pub(super) fn toggle(&mut self, value: u64) {
        self.0 ^= value;
    }
}

pub(super) fn recompute(position: &Position) -> RepetitionKey {
    let mut key = RepetitionKey::ZERO;
    for color in Color::ALL {
        for kind in PieceKind::ALL {
            for square in position.bitboard(color, kind).squares() {
                key.toggle(piece_key(color, kind, square));
            }
        }
    }
    if position.side_to_move() == Color::Black {
        key.toggle(side_key());
    }
    for (color, side) in castling_permissions() {
        if position.allows_castling(color, side) {
            key.toggle(castling_key(color, side));
        }
    }
    if let Some(file) = legal_en_passant_file(position) {
        key.toggle(en_passant_key(file));
    }
    key
}

pub(super) const fn piece_key(color: Color, kind: PieceKind, square: Square) -> u64 {
    let piece = color as u64 * PieceKind::COUNT as u64 + kind as u64;
    random_key(piece * 64 + square.index() as u64)
}

pub(super) const fn side_key() -> u64 {
    random_key(SIDE_KEY)
}

pub(super) const fn castling_key(color: Color, side: CastleSide) -> u64 {
    let offset = match (color, side) {
        (Color::White, CastleSide::KingSide) => 0,
        (Color::White, CastleSide::QueenSide) => 1,
        (Color::Black, CastleSide::KingSide) => 2,
        (Color::Black, CastleSide::QueenSide) => 3,
    };
    random_key(CASTLING_KEYS + offset)
}

pub(super) const fn en_passant_key(file: u8) -> u64 {
    random_key(EN_PASSANT_KEYS + file as u64)
}

pub(super) fn legal_en_passant_file(position: &Position) -> Option<u8> {
    let target = position.en_passant()?;
    let color = position.side_to_move();
    let source_rank = match color {
        Color::White => target.rank().checked_sub(1)?,
        Color::Black => target.rank().checked_add(1)?,
    };
    let captured = Square::new(target.file(), source_rank)?;
    if position.piece_at(captured) != Some(super::Piece::new(color.opposite(), PieceKind::Pawn)) {
        return None;
    }

    for file_delta in [-1_i8, 1] {
        let source_file = i16::from(target.file()) + i16::from(file_delta);
        if !(0..8).contains(&source_file) {
            continue;
        }
        let source = Square::new(
            u8::try_from(source_file).expect("checked file is on board"),
            source_rank,
        )
        .expect("checked coordinates are on board");
        if position.piece_at(source) != Some(super::Piece::new(color, PieceKind::Pawn)) {
            continue;
        }

        let mut after = position.clone();
        after.pieces[color.index()][PieceKind::Pawn.index()].remove(source);
        after.pieces[color.opposite().index()][PieceKind::Pawn.index()].remove(captured);
        after.pieces[color.index()][PieceKind::Pawn.index()].insert(target);
        let mut kings = after.bitboard(color, PieceKind::King).squares();
        let king = kings.next()?;
        if kings.next().is_none() && !is_square_attacked(&after, king, color.opposite()) {
            return Some(target.file());
        }
    }
    None
}

const fn castling_permissions() -> [(Color, CastleSide); 4] {
    [
        (Color::White, CastleSide::KingSide),
        (Color::White, CastleSide::QueenSide),
        (Color::Black, CastleSide::KingSide),
        (Color::Black, CastleSide::QueenSide),
    ]
}

const fn random_key(index: u64) -> u64 {
    splitmix64(SEED.wrapping_add(index))
}

const fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::recompute;
    use crate::chess::Position;

    #[test]
    fn clocks_do_not_affect_repetition_identity() {
        let first = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").expect("valid FEN");
        let second = Position::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 99 42").expect("valid FEN");

        assert_eq!(recompute(&first), recompute(&second));
    }

    #[test]
    fn side_to_move_and_castling_rights_affect_identity() {
        let white = Position::from_fen("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1").expect("valid FEN");
        let black = Position::from_fen("4k3/8/8/8/8/8/8/R3K2R b KQ - 0 1").expect("valid FEN");
        let no_castling = Position::from_fen("4k3/8/8/8/8/8/8/R3K2R w - - 0 1").expect("valid FEN");

        assert_ne!(recompute(&white), recompute(&black));
        assert_ne!(recompute(&white), recompute(&no_castling));
    }

    #[test]
    fn uncapturable_en_passant_target_does_not_affect_identity() {
        let with_target =
            Position::from_fen("4k3/8/8/3p4/8/8/8/4K3 w - d6 0 2").expect("valid FEN");
        let without_target =
            Position::from_fen("4k3/8/8/3p4/8/8/8/4K3 w - - 0 2").expect("valid FEN");

        assert_eq!(recompute(&with_target), recompute(&without_target));
    }

    #[test]
    fn pinned_en_passant_capture_does_not_affect_identity() {
        let with_target =
            Position::from_fen("k3r3/8/8/3pP3/8/8/8/4K3 w - d6 0 2").expect("valid FEN");
        let without_target =
            Position::from_fen("k3r3/8/8/3pP3/8/8/8/4K3 w - - 0 2").expect("valid FEN");

        assert_eq!(recompute(&with_target), recompute(&without_target));
    }

    #[test]
    fn legal_en_passant_capture_changes_identity() {
        let with_target =
            Position::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 2").expect("valid FEN");
        let without_target =
            Position::from_fen("4k3/8/8/3pP3/8/8/8/4K3 w - - 0 2").expect("valid FEN");

        assert_ne!(recompute(&with_target), recompute(&without_target));
    }
}
