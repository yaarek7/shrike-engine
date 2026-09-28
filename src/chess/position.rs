use super::{
    Bitboard, CastleSide, CastlingRights, Color, FenError, Move, MoveKind, Piece, PieceKind,
    RepetitionKey, Square, zobrist,
};

/// State required to restore a position after making a move.
///
/// M3 intentionally snapshots the position for correctness. A later measured
/// optimization may replace this with a smaller incremental undo record.
/// An undo record must be returned to the same [`Position`] in last-in,
/// first-out order.
#[derive(Debug, PartialEq, Eq)]
pub struct Undo {
    previous: Position,
}

/// A complete chess position state as represented by FEN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub(super) pieces: [[Bitboard; PieceKind::COUNT]; Color::COUNT],
    pub(super) side_to_move: Color,
    pub(super) castling_rights: CastlingRights,
    pub(super) en_passant: Option<Square>,
    pub(super) halfmove_clock: u32,
    pub(super) fullmove_number: u32,
    pub(super) repetition_key: RepetitionKey,
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

    /// Returns the deterministic key used to compare positions for repetition.
    ///
    /// This excludes move clocks and includes an en-passant file only when a
    /// legal en-passant capture exists. It is not a transposition-table key.
    #[must_use]
    pub const fn repetition_key(&self) -> RepetitionKey {
        self.repetition_key
    }

    /// Conservatively identifies common positions where checkmate is impossible.
    ///
    /// This recognizes king versus king, king and a single bishop or knight
    /// versus king, and bishops-only positions where every bishop occupies the
    /// same square color. It is intentionally not a complete FIDE dead-position
    /// detector.
    #[must_use]
    pub fn has_insufficient_material(&self) -> bool {
        for color in Color::ALL {
            if self.bitboard(color, PieceKind::Pawn).bits() != 0
                || self.bitboard(color, PieceKind::Rook).bits() != 0
                || self.bitboard(color, PieceKind::Queen).bits() != 0
            {
                return false;
            }
        }

        let knights = self.bitboard(Color::White, PieceKind::Knight).count()
            + self.bitboard(Color::Black, PieceKind::Knight).count();
        let bishops = self.bitboard(Color::White, PieceKind::Bishop).count()
            + self.bitboard(Color::Black, PieceKind::Bishop).count();
        if bishops == 0 {
            return knights <= 1;
        }
        if knights != 0 {
            return false;
        }

        let all_bishops = self.bitboard(Color::White, PieceKind::Bishop).bits()
            | self.bitboard(Color::Black, PieceKind::Bishop).bits();
        let light_squares = 0x55AA_55AA_55AA_55AA_u64;
        all_bishops & light_squares == 0 || all_bishops & !light_squares == 0
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

    pub(crate) fn apply_move_unchecked(&mut self, chess_move: Move) -> Undo {
        let undo = Undo {
            previous: self.clone(),
        };
        if let Some(file) = zobrist::legal_en_passant_file(self) {
            self.repetition_key.toggle(zobrist::en_passant_key(file));
        }
        let from = chess_move.from();
        let to = chess_move.to();
        let moving_piece = self
            .piece_at(from)
            .expect("generated moves always have a source piece");
        self.repetition_key.toggle(zobrist::piece_key(
            moving_piece.color,
            moving_piece.kind,
            from,
        ));
        let captured_piece = if chess_move.kind() == MoveKind::EnPassant {
            let captured_rank = match moving_piece.color {
                Color::White => to.rank() - 1,
                Color::Black => to.rank() + 1,
            };
            let captured_square =
                Square::new(to.file(), captured_rank).expect("en-passant capture is on board");
            let piece = self.piece_at(captured_square);
            if let Some(piece) = piece {
                self.remove_piece(piece, captured_square);
                self.repetition_key.toggle(zobrist::piece_key(
                    piece.color,
                    piece.kind,
                    captured_square,
                ));
            }
            piece.map(|piece| (piece, captured_square))
        } else {
            let piece = self.piece_at(to);
            if let Some(piece) = piece {
                self.remove_piece(piece, to);
                self.repetition_key
                    .toggle(zobrist::piece_key(piece.color, piece.kind, to));
            }
            piece.map(|piece| (piece, to))
        };

        self.remove_piece(moving_piece, from);
        if matches!(
            chess_move.kind(),
            MoveKind::KingCastle | MoveKind::QueenCastle
        ) {
            self.move_castling_rook(moving_piece.color, chess_move.kind());
        }

        let placed_kind = chess_move.promotion().unwrap_or(moving_piece.kind);
        self.insert_piece(Piece::new(moving_piece.color, placed_kind), to);
        self.repetition_key
            .toggle(zobrist::piece_key(moving_piece.color, placed_kind, to));

        let previous_castling = self.castling_rights;
        self.update_castling_rights(moving_piece, from, captured_piece);
        for color in Color::ALL {
            for side in [CastleSide::KingSide, CastleSide::QueenSide] {
                if previous_castling.allows(color, side) != self.castling_rights.allows(color, side)
                {
                    self.repetition_key
                        .toggle(zobrist::castling_key(color, side));
                }
            }
        }
        self.en_passant = if chess_move.kind() == MoveKind::DoublePawnPush {
            let passed_rank = match moving_piece.color {
                Color::White => from.rank() + 1,
                Color::Black => from.rank() - 1,
            };
            Square::new(from.file(), passed_rank)
        } else {
            None
        };
        self.halfmove_clock = if moving_piece.kind == PieceKind::Pawn || captured_piece.is_some() {
            0
        } else {
            self.halfmove_clock.saturating_add(1)
        };
        if self.side_to_move == Color::Black {
            self.fullmove_number = self.fullmove_number.saturating_add(1);
        }
        self.side_to_move = self.side_to_move.opposite();
        self.repetition_key.toggle(zobrist::side_key());
        if let Some(file) = zobrist::legal_en_passant_file(self) {
            self.repetition_key.toggle(zobrist::en_passant_key(file));
        }
        debug_assert_eq!(self.repetition_key, zobrist::recompute(self));
        undo
    }

    /// Restores the exact position captured before a move was made.
    pub fn unmake_move(&mut self, undo: Undo) {
        *self = undo.previous;
    }

    fn insert_piece(&mut self, piece: Piece, square: Square) {
        self.pieces[piece.color.index()][piece.kind.index()].insert(square);
    }

    fn remove_piece(&mut self, piece: Piece, square: Square) {
        self.pieces[piece.color.index()][piece.kind.index()].remove(square);
    }

    fn move_castling_rook(&mut self, color: Color, kind: MoveKind) {
        let rank = match color {
            Color::White => 0,
            Color::Black => 7,
        };
        let (from_file, to_file) = match kind {
            MoveKind::KingCastle => (7, 5),
            MoveKind::QueenCastle => (0, 3),
            _ => unreachable!(),
        };
        let from = Square::new(from_file, rank).expect("castling rook source is on board");
        let to = Square::new(to_file, rank).expect("castling rook destination is on board");
        let rook = Piece::new(color, PieceKind::Rook);
        self.remove_piece(rook, from);
        self.insert_piece(rook, to);
        self.repetition_key
            .toggle(zobrist::piece_key(color, PieceKind::Rook, from));
        self.repetition_key
            .toggle(zobrist::piece_key(color, PieceKind::Rook, to));
    }

    fn update_castling_rights(
        &mut self,
        moving_piece: Piece,
        from: Square,
        captured_piece: Option<(Piece, Square)>,
    ) {
        if moving_piece.kind == PieceKind::King {
            self.castling_rights.revoke_all(moving_piece.color);
        } else if moving_piece.kind == PieceKind::Rook {
            self.revoke_rook_right(moving_piece.color, from);
        }
        if let Some((piece, square)) = captured_piece {
            if piece.kind == PieceKind::Rook {
                self.revoke_rook_right(piece.color, square);
            }
        }
    }

    fn revoke_rook_right(&mut self, color: Color, square: Square) {
        let rank = match color {
            Color::White => 0,
            Color::Black => 7,
        };
        if square == Square::new(0, rank).expect("rook home square is on board") {
            self.castling_rights.revoke(color, CastleSide::QueenSide);
        } else if square == Square::new(7, rank).expect("rook home square is on board") {
            self.castling_rights.revoke(color, CastleSide::KingSide);
        }
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::starting()
    }
}

#[cfg(test)]
mod tests {
    use super::Position;

    #[test]
    fn conservative_insufficient_material_cases_are_recognized() {
        for fen in [
            "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/2B1K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/2N1K3 w - - 0 1",
            "4kb2/8/8/8/8/8/8/2B1K3 w - - 0 1",
            "4k3/8/8/6b1/8/4B3/8/4K3 w - - 0 1",
        ] {
            let position = Position::from_fen(fen).expect("valid FEN");
            assert!(position.has_insufficient_material(), "expected draw: {fen}");
        }
    }

    #[test]
    fn potentially_mating_material_is_not_declared_insufficient() {
        for fen in [
            "4k3/8/8/8/8/8/8/R3K3 w - - 0 1",
            "4k3/8/8/8/8/8/8/2NNK3 w - - 0 1",
            "4k3/8/8/8/8/8/8/2BNK3 w - - 0 1",
            "2b1k3/8/8/8/8/8/8/2B1K3 w - - 0 1",
        ] {
            let position = Position::from_fen(fen).expect("valid FEN");
            assert!(
                !position.has_insufficient_material(),
                "must remain searchable: {fen}"
            );
        }
    }
}
