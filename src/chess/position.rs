use super::{Bitboard, CastleSide, CastlingRights, Color, FenError, Piece, PieceKind, Square};

/// A complete chess position state as represented by FEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub(super) pieces: [[Bitboard; PieceKind::COUNT]; Color::COUNT],
    pub(super) side_to_move: Color,
    pub(super) castling_rights: CastlingRights,
    pub(super) en_passant: Option<Square>,
    pub(super) halfmove_clock: u32,
    pub(super) fullmove_number: u32,
}

impl Position {
    /// The standard chess starting position in FEN notation.
    pub const STARTING_FEN: &'static str =
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    /// Parses a position from Forsyth-Edwards Notation (FEN).
    ///
    /// This validates FEN structure. Full chess legality validation will be
    /// introduced alongside attack detection and legal move generation.
    ///
    /// # Errors
    ///
    /// Returns [`FenError`] when the input does not satisfy the supported FEN
    /// structure or one of its fields is malformed.
    pub fn from_fen(fen: &str) -> Result<Self, FenError> {
        fen.parse()
    }

    /// Returns the standard chess starting position.
    ///
    /// # Panics
    ///
    /// Panics only if the compile-time [`Self::STARTING_FEN`] constant violates
    /// the parser's invariants, which indicates a defect in this crate.
    #[must_use]
    pub fn starting() -> Self {
        Self::from_fen(Self::STARTING_FEN).expect("the built-in starting FEN must be valid")
    }

    /// Serializes this position as canonical FEN.
    #[must_use]
    pub fn to_fen(&self) -> String {
        self.to_string()
    }

    /// Returns the side whose turn it is.
    #[must_use]
    pub const fn side_to_move(&self) -> Color {
        self.side_to_move
    }

    /// Returns the position's castling rights.
    #[must_use]
    pub const fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    /// Returns the en-passant target square, if one is recorded.
    #[must_use]
    pub const fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    /// Returns the halfmove clock used by the fifty-move rule.
    #[must_use]
    pub const fn halfmove_clock(&self) -> u32 {
        self.halfmove_clock
    }

    /// Returns the one-based fullmove number.
    #[must_use]
    pub const fn fullmove_number(&self) -> u32 {
        self.fullmove_number
    }

    /// Returns the pieces matching `color` and `kind`.
    #[must_use]
    pub const fn bitboard(&self, color: Color, kind: PieceKind) -> Bitboard {
        self.pieces[color.index()][kind.index()]
    }

    /// Returns every occupied square for `color`.
    #[must_use]
    pub fn occupancy(&self, color: Color) -> Bitboard {
        let mut bits = 0;
        for kind in PieceKind::ALL {
            bits |= self.bitboard(color, kind).bits();
        }
        Bitboard::from_bits(bits)
    }

    /// Returns every occupied square.
    #[must_use]
    pub fn occupied(&self) -> Bitboard {
        Bitboard::from_bits(
            self.occupancy(Color::White).bits() | self.occupancy(Color::Black).bits(),
        )
    }

    /// Returns the piece occupying `square`, or `None` if it is empty.
    #[must_use]
    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                if self.bitboard(color, kind).contains(square) {
                    return Some(Piece::new(color, kind));
                }
            }
        }
        None
    }

    /// Returns whether the given castling permission is recorded.
    #[must_use]
    pub const fn allows_castling(&self, color: Color, side: CastleSide) -> bool {
        self.castling_rights.allows(color, side)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::starting()
    }
}
