use std::fmt;

use super::{PieceKind, Square};

const SQUARE_MASK: u16 = 0x3f;
const TO_SHIFT: u32 = 6;
const KIND_SHIFT: u32 = 12;

/// The semantic category encoded in a chess move.
///
/// The discriminants are part of [`Move`]'s stable internal representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MoveKind {
    /// A non-capturing move with no additional effect.
    Quiet = 0,
    /// A pawn advancing two squares from its starting rank.
    DoublePawnPush = 1,
    /// King-side castling.
    KingCastle = 2,
    /// Queen-side castling.
    QueenCastle = 3,
    /// An ordinary capture.
    Capture = 4,
    /// An en-passant capture.
    EnPassant = 5,
    /// A non-capturing promotion to a knight.
    KnightPromotion = 8,
    /// A non-capturing promotion to a bishop.
    BishopPromotion = 9,
    /// A non-capturing promotion to a rook.
    RookPromotion = 10,
    /// A non-capturing promotion to a queen.
    QueenPromotion = 11,
    /// A capturing promotion to a knight.
    KnightPromotionCapture = 12,
    /// A capturing promotion to a bishop.
    BishopPromotionCapture = 13,
    /// A capturing promotion to a rook.
    RookPromotionCapture = 14,
    /// A capturing promotion to a queen.
    QueenPromotionCapture = 15,
}

impl MoveKind {
    /// Returns whether this move kind captures an opposing piece.
    #[must_use]
    pub const fn is_capture(self) -> bool {
        matches!(
            self,
            Self::Capture
                | Self::EnPassant
                | Self::KnightPromotionCapture
                | Self::BishopPromotionCapture
                | Self::RookPromotionCapture
                | Self::QueenPromotionCapture
        )
    }

    /// Returns the promoted piece kind, or `None` for a non-promotion.
    #[must_use]
    pub const fn promotion(self) -> Option<PieceKind> {
        match self {
            Self::KnightPromotion | Self::KnightPromotionCapture => Some(PieceKind::Knight),
            Self::BishopPromotion | Self::BishopPromotionCapture => Some(PieceKind::Bishop),
            Self::RookPromotion | Self::RookPromotionCapture => Some(PieceKind::Rook),
            Self::QueenPromotion | Self::QueenPromotionCapture => Some(PieceKind::Queen),
            Self::Quiet
            | Self::DoublePawnPush
            | Self::KingCastle
            | Self::QueenCastle
            | Self::Capture
            | Self::EnPassant => None,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Quiet),
            1 => Some(Self::DoublePawnPush),
            2 => Some(Self::KingCastle),
            3 => Some(Self::QueenCastle),
            4 => Some(Self::Capture),
            5 => Some(Self::EnPassant),
            8 => Some(Self::KnightPromotion),
            9 => Some(Self::BishopPromotion),
            10 => Some(Self::RookPromotion),
            11 => Some(Self::QueenPromotion),
            12 => Some(Self::KnightPromotionCapture),
            13 => Some(Self::BishopPromotionCapture),
            14 => Some(Self::RookPromotionCapture),
            15 => Some(Self::QueenPromotionCapture),
            _ => None,
        }
    }
}

/// A compact, two-byte chess move.
///
/// Bits 0–5 encode the source square, bits 6–11 the destination square, and
/// bits 12–15 the [`MoveKind`]. The value records move semantics but does not
/// independently establish legality in a particular position.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Move(u16);

impl Move {
    /// Creates a move, returning `None` when the source equals the destination.
    #[must_use]
    pub fn new(from: Square, to: Square, kind: MoveKind) -> Option<Self> {
        if from.index() == to.index() {
            return None;
        }

        Some(Self(
            u16::from(from.index())
                | (u16::from(to.index()) << TO_SHIFT)
                | ((kind as u16) << KIND_SHIFT),
        ))
    }

    /// Reconstructs a move from its raw representation.
    ///
    /// Returns `None` for reserved move-kind codes or a null move. Search-level
    /// null moves will use a distinct representation when introduced.
    #[must_use]
    pub fn from_raw(raw: u16) -> Option<Self> {
        let from = (raw & SQUARE_MASK) as u8;
        let to = ((raw >> TO_SHIFT) & SQUARE_MASK) as u8;
        let kind = (raw >> KIND_SHIFT) as u8;
        if from == to || MoveKind::from_code(kind).is_none() {
            None
        } else {
            Some(Self(raw))
        }
    }

    /// Returns the stable raw representation.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Returns the source square.
    #[must_use]
    pub fn from(self) -> Square {
        // Every six-bit square index is in 0..64.
        match Square::from_index((self.0 & SQUARE_MASK) as u8) {
            Some(square) => square,
            None => unreachable!(),
        }
    }

    /// Returns the destination square.
    #[must_use]
    pub fn to(self) -> Square {
        // Every six-bit square index is in 0..64.
        match Square::from_index(((self.0 >> TO_SHIFT) & SQUARE_MASK) as u8) {
            Some(square) => square,
            None => unreachable!(),
        }
    }

    /// Returns the move's semantic category.
    #[must_use]
    pub fn kind(self) -> MoveKind {
        match MoveKind::from_code((self.0 >> KIND_SHIFT) as u8) {
            Some(kind) => kind,
            None => unreachable!(),
        }
    }

    /// Returns whether this move captures an opposing piece.
    #[must_use]
    pub fn is_capture(self) -> bool {
        self.kind().is_capture()
    }

    /// Returns the promoted piece kind, or `None` for a non-promotion.
    #[must_use]
    pub fn promotion(self) -> Option<PieceKind> {
        self.kind().promotion()
    }

    /// Writes this move in UCI coordinate notation.
    #[must_use]
    pub fn to_uci(self) -> String {
        self.to_string()
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Move")
            .field("uci", &self.to_string())
            .field("kind", &self.kind())
            .finish()
    }
}

impl fmt::Display for Move {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.from(), self.to())?;
        if let Some(piece) = self.promotion() {
            formatter.write_str(match piece {
                PieceKind::Knight => "n",
                PieceKind::Bishop => "b",
                PieceKind::Rook => "r",
                PieceKind::Queen => "q",
                PieceKind::Pawn | PieceKind::King => unreachable!(),
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;

    use super::{KIND_SHIFT, Move, MoveKind, TO_SHIFT};
    use crate::chess::{PieceKind, Square};

    const ALL_KINDS: [MoveKind; 14] = [
        MoveKind::Quiet,
        MoveKind::DoublePawnPush,
        MoveKind::KingCastle,
        MoveKind::QueenCastle,
        MoveKind::Capture,
        MoveKind::EnPassant,
        MoveKind::KnightPromotion,
        MoveKind::BishopPromotion,
        MoveKind::RookPromotion,
        MoveKind::QueenPromotion,
        MoveKind::KnightPromotionCapture,
        MoveKind::BishopPromotionCapture,
        MoveKind::RookPromotionCapture,
        MoveKind::QueenPromotionCapture,
    ];

    #[test]
    fn move_occupies_two_bytes() {
        assert_eq!(size_of::<Move>(), 2);
    }

    #[test]
    fn every_move_kind_round_trips_through_raw_encoding() {
        for from_index in 0..64 {
            for to_index in 0..64 {
                if from_index == to_index {
                    continue;
                }
                let from = Square::from_index(from_index).expect("index is on board");
                let to = Square::from_index(to_index).expect("index is on board");
                for kind in ALL_KINDS {
                    let chess_move =
                        Move::new(from, to, kind).expect("different squares form a move");
                    assert_eq!(chess_move.from(), from);
                    assert_eq!(chess_move.to(), to);
                    assert_eq!(chess_move.kind(), kind);
                    assert_eq!(Move::from_raw(chess_move.raw()), Some(chess_move));
                }
            }
        }
    }

    #[test]
    fn reserved_kinds_and_null_moves_are_rejected() {
        let distinct_squares = 1_u16 << TO_SHIFT;
        assert_eq!(
            Move::from_raw(distinct_squares | (6_u16 << KIND_SHIFT)),
            None
        );
        assert_eq!(
            Move::from_raw(distinct_squares | (7_u16 << KIND_SHIFT)),
            None
        );

        let e4 = Square::new(4, 3).expect("E4 is valid");
        assert_eq!(Move::new(e4, e4, MoveKind::Quiet), None);
    }

    #[test]
    fn capture_and_promotion_properties_are_derived_from_kind() {
        assert!(MoveKind::Capture.is_capture());
        assert!(MoveKind::EnPassant.is_capture());
        assert!(MoveKind::QueenPromotionCapture.is_capture());
        assert!(!MoveKind::QueenPromotion.is_capture());
        assert_eq!(
            MoveKind::KnightPromotion.promotion(),
            Some(PieceKind::Knight)
        );
        assert_eq!(MoveKind::Quiet.promotion(), None);
    }

    #[test]
    fn uci_notation_includes_promotion_piece() {
        let a7 = Square::new(0, 6).expect("A7 is valid");
        let a8 = Square::new(0, 7).expect("A8 is valid");
        let promotion =
            Move::new(a7, a8, MoveKind::KnightPromotion).expect("different squares form a move");

        assert_eq!(promotion.to_uci(), "a7a8n");
    }
}
