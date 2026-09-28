use std::{error::Error, fmt, str::FromStr};

use super::{Bitboard, CastleSide, CastlingRights, Color, Piece, PieceKind, Position, Square};

/// An error produced while parsing a FEN position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FenError {
    /// FEN must contain exactly six space-delimited fields.
    FieldCount {
        /// Number of fields found.
        found: usize,
    },
    /// Piece placement must contain exactly eight ranks.
    RankCount {
        /// Number of ranks found.
        found: usize,
    },
    /// A rank did not describe exactly eight files.
    RankWidth {
        /// Human-facing rank number in `1..=8`.
        rank: u8,
        /// Number of files described.
        found: u8,
    },
    /// The piece-placement field contained an unknown character.
    InvalidPiece(char),
    /// The active-color field was not `w` or `b`.
    InvalidSideToMove(String),
    /// The castling field was malformed or repeated a permission.
    InvalidCastlingRights(String),
    /// The en-passant field was not `-` or a square on rank 3 or 6.
    InvalidEnPassant(String),
    /// The halfmove clock was not a non-negative integer.
    InvalidHalfmoveClock(String),
    /// The fullmove number was not a positive integer.
    InvalidFullmoveNumber(String),
}

impl fmt::Display for FenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FieldCount { found } => {
                write!(formatter, "FEN must contain 6 fields, found {found}")
            }
            Self::RankCount { found } => {
                write!(
                    formatter,
                    "piece placement must contain 8 ranks, found {found}"
                )
            }
            Self::RankWidth { rank, found } => {
                write!(formatter, "rank {rank} must contain 8 files, found {found}")
            }
            Self::InvalidPiece(piece) => write!(formatter, "invalid FEN piece '{piece}'"),
            Self::InvalidSideToMove(value) => write!(formatter, "invalid side to move '{value}'"),
            Self::InvalidCastlingRights(value) => {
                write!(formatter, "invalid castling rights '{value}'")
            }
            Self::InvalidEnPassant(value) => {
                write!(formatter, "invalid en-passant target '{value}'")
            }
            Self::InvalidHalfmoveClock(value) => {
                write!(formatter, "invalid halfmove clock '{value}'")
            }
            Self::InvalidFullmoveNumber(value) => {
                write!(formatter, "invalid fullmove number '{value}'")
            }
        }
    }
}

impl Error for FenError {}

impl FromStr for Position {
    type Err = FenError;

    fn from_str(fen: &str) -> Result<Self, Self::Err> {
        let fields: Vec<_> = fen.split_whitespace().collect();
        if fields.len() != 6 {
            return Err(FenError::FieldCount {
                found: fields.len(),
            });
        }

        let pieces = parse_piece_placement(fields[0])?;
        let side_to_move = parse_side_to_move(fields[1])?;
        let castling_rights = parse_castling_rights(fields[2])?;
        let en_passant = parse_en_passant(fields[3])?;
        let halfmove_clock = fields[4]
            .parse()
            .map_err(|_| FenError::InvalidHalfmoveClock(fields[4].to_owned()))?;
        let fullmove_number = fields[5]
            .parse()
            .map_err(|_| FenError::InvalidFullmoveNumber(fields[5].to_owned()))?;
        if fullmove_number == 0 {
            return Err(FenError::InvalidFullmoveNumber(fields[5].to_owned()));
        }

        Ok(Self {
            pieces,
            side_to_move,
            castling_rights,
            en_passant,
            halfmove_clock,
            fullmove_number,
        })
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_piece_placement(self, formatter)?;
        write!(
            formatter,
            " {} ",
            match self.side_to_move() {
                Color::White => 'w',
                Color::Black => 'b',
            }
        )?;
        write_castling_rights(self.castling_rights(), formatter)?;
        write!(
            formatter,
            " {} {} {}",
            self.en_passant()
                .map_or_else(|| "-".to_owned(), |square| square.to_string()),
            self.halfmove_clock(),
            self.fullmove_number()
        )
    }
}

fn parse_piece_placement(
    placement: &str,
) -> Result<[[Bitboard; PieceKind::COUNT]; Color::COUNT], FenError> {
    let ranks: Vec<_> = placement.split('/').collect();
    if ranks.len() != 8 {
        return Err(FenError::RankCount { found: ranks.len() });
    }

    let mut pieces = [[Bitboard::EMPTY; PieceKind::COUNT]; Color::COUNT];
    for (fen_rank, rank_text) in (0_u8..).zip(ranks) {
        let board_rank = 7 - fen_rank;
        let mut file = 0_u8;

        for symbol in rank_text.chars() {
            if let Some(empty_count) = symbol.to_digit(10) {
                if !(1..=8).contains(&empty_count) {
                    return Err(FenError::RankWidth {
                        rank: board_rank + 1,
                        found: file,
                    });
                }
                let empty_count = u8::try_from(empty_count).expect("a digit in 1..=8 fits in u8");
                file = file.checked_add(empty_count).ok_or(FenError::RankWidth {
                    rank: board_rank + 1,
                    found: u8::MAX,
                })?;
            } else {
                let piece = piece_from_symbol(symbol).ok_or(FenError::InvalidPiece(symbol))?;
                if file >= 8 {
                    return Err(FenError::RankWidth {
                        rank: board_rank + 1,
                        found: file + 1,
                    });
                }
                let square =
                    Square::new(file, board_rank).expect("parsed coordinates are on board");
                pieces[piece.color.index()][piece.kind.index()].insert(square);
                file += 1;
            }
        }

        if file != 8 {
            return Err(FenError::RankWidth {
                rank: board_rank + 1,
                found: file,
            });
        }
    }

    Ok(pieces)
}

fn parse_side_to_move(value: &str) -> Result<Color, FenError> {
    match value {
        "w" => Ok(Color::White),
        "b" => Ok(Color::Black),
        _ => Err(FenError::InvalidSideToMove(value.to_owned())),
    }
}

fn parse_castling_rights(value: &str) -> Result<CastlingRights, FenError> {
    if value == "-" {
        return Ok(CastlingRights::NONE);
    }
    if value.is_empty() || value.contains('-') {
        return Err(FenError::InvalidCastlingRights(value.to_owned()));
    }

    let mut rights = CastlingRights::NONE;
    let mut seen = [false; 4];
    for symbol in value.chars() {
        let (index, color, side) = match symbol {
            'K' => (0, Color::White, CastleSide::KingSide),
            'Q' => (1, Color::White, CastleSide::QueenSide),
            'k' => (2, Color::Black, CastleSide::KingSide),
            'q' => (3, Color::Black, CastleSide::QueenSide),
            _ => return Err(FenError::InvalidCastlingRights(value.to_owned())),
        };
        if seen[index] {
            return Err(FenError::InvalidCastlingRights(value.to_owned()));
        }
        seen[index] = true;
        rights.grant(color, side);
    }

    Ok(rights)
}

fn parse_en_passant(value: &str) -> Result<Option<Square>, FenError> {
    if value == "-" {
        return Ok(None);
    }

    let square = value
        .parse::<Square>()
        .map_err(|_| FenError::InvalidEnPassant(value.to_owned()))?;
    if !matches!(square.rank(), 2 | 5) {
        return Err(FenError::InvalidEnPassant(value.to_owned()));
    }

    Ok(Some(square))
}

fn piece_from_symbol(symbol: char) -> Option<Piece> {
    let color = if symbol.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    let kind = match symbol.to_ascii_lowercase() {
        'p' => PieceKind::Pawn,
        'n' => PieceKind::Knight,
        'b' => PieceKind::Bishop,
        'r' => PieceKind::Rook,
        'q' => PieceKind::Queen,
        'k' => PieceKind::King,
        _ => return None,
    };
    Some(Piece::new(color, kind))
}

fn piece_symbol(piece: Piece) -> char {
    let symbol = match piece.kind {
        PieceKind::Pawn => 'p',
        PieceKind::Knight => 'n',
        PieceKind::Bishop => 'b',
        PieceKind::Rook => 'r',
        PieceKind::Queen => 'q',
        PieceKind::King => 'k',
    };
    match piece.color {
        Color::White => symbol.to_ascii_uppercase(),
        Color::Black => symbol,
    }
}

fn write_piece_placement(position: &Position, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    for rank in (0_u8..8).rev() {
        if rank != 7 {
            formatter.write_str("/")?;
        }
        let mut empty = 0;
        for file in 0_u8..8 {
            let square = Square::new(file, rank).expect("loop coordinates are on board");
            if let Some(piece) = position.piece_at(square) {
                if empty != 0 {
                    write!(formatter, "{empty}")?;
                    empty = 0;
                }
                write!(formatter, "{}", piece_symbol(piece))?;
            } else {
                empty += 1;
            }
        }
        if empty != 0 {
            write!(formatter, "{empty}")?;
        }
    }
    Ok(())
}

fn write_castling_rights(
    rights: CastlingRights,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if rights.is_empty() {
        return formatter.write_str("-");
    }
    for (color, side, symbol) in [
        (Color::White, CastleSide::KingSide, 'K'),
        (Color::White, CastleSide::QueenSide, 'Q'),
        (Color::Black, CastleSide::KingSide, 'k'),
        (Color::Black, CastleSide::QueenSide, 'q'),
    ] {
        if rights.allows(color, side) {
            write!(formatter, "{symbol}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::FenError;
    use crate::chess::{CastleSide, Color, Piece, PieceKind, Position, Square};

    #[test]
    fn starting_position_has_expected_state() {
        let position = Position::starting();

        assert_eq!(position.side_to_move(), Color::White);
        assert_eq!(position.occupancy(Color::White).count(), 16);
        assert_eq!(position.occupancy(Color::Black).count(), 16);
        assert_eq!(position.occupied().count(), 32);
        assert!(position.allows_castling(Color::White, CastleSide::KingSide));
        assert!(position.allows_castling(Color::White, CastleSide::QueenSide));
        assert!(position.allows_castling(Color::Black, CastleSide::KingSide));
        assert!(position.allows_castling(Color::Black, CastleSide::QueenSide));
        assert_eq!(position.en_passant(), None);
        assert_eq!(position.halfmove_clock(), 0);
        assert_eq!(position.fullmove_number(), 1);

        let e1: Square = "e1".parse().expect("valid square");
        assert_eq!(
            position.piece_at(e1),
            Some(Piece::new(Color::White, PieceKind::King))
        );
    }

    #[test]
    fn canonical_positions_round_trip_exactly() {
        for fen in [
            Position::STARTING_FEN,
            "8/8/8/8/8/8/8/8 w - - 0 1",
            "r3k2r/pppq1ppp/2npbn2/3Np3/2B1P3/2N2Q1P/PPP2PP1/R3K2R b KQkq e3 7 12",
            "4k3/8/8/8/3Pp3/8/8/4K3 w - e6 99 250",
        ] {
            let position = Position::from_fen(fen).expect("test FEN should parse");
            assert_eq!(position.to_fen(), fen);
        }
    }

    #[test]
    fn serializer_canonicalizes_castling_order() {
        let position = Position::from_fen("8/8/8/8/8/8/8/8 w qK - 0 1")
            .expect("unordered unique rights are accepted");

        assert_eq!(position.to_fen(), "8/8/8/8/8/8/8/8 w Kq - 0 1");
    }

    #[test]
    fn malformed_fen_is_rejected() {
        let invalid = [
            "8/8/8/8/8/8/8/8 w - - 0",
            "8/8/8/8/8/8/8 w - - 0 1",
            "8/8/8/8/8/8/8/9 w - - 0 1",
            "8/8/8/8/8/8/8/7x w - - 0 1",
            "8/8/8/8/8/8/8/8 x - - 0 1",
            "8/8/8/8/8/8/8/8 w KK - 0 1",
            "8/8/8/8/8/8/8/8 w - e4 0 1",
            "8/8/8/8/8/8/8/8 w - - x 1",
            "8/8/8/8/8/8/8/8 w - - 0 0",
        ];

        for fen in invalid {
            assert!(
                Position::from_fen(fen).is_err(),
                "accepted invalid FEN: {fen}"
            );
        }
    }

    #[test]
    fn field_count_error_reports_observed_count() {
        assert_eq!(
            Position::from_fen("8/8/8/8/8/8/8/8 w - - 0"),
            Err(FenError::FieldCount { found: 5 })
        );
    }
}
