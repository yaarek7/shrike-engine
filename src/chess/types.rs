use std::{error::Error, fmt, str::FromStr};

/// The color of a chess piece or side to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Color {
    /// White.
    White = 0,
    /// Black.
    Black = 1,
}

impl Color {
    pub(super) const COUNT: usize = 2;
    pub(super) const ALL: [Self; Self::COUNT] = [Self::White, Self::Black];

    /// Returns the opposing color.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    pub(super) const fn index(self) -> usize {
        self as usize
    }
}

/// A kind of chess piece, independent of color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PieceKind {
    /// Pawn.
    Pawn = 0,
    /// Knight.
    Knight = 1,
    /// Bishop.
    Bishop = 2,
    /// Rook.
    Rook = 3,
    /// Queen.
    Queen = 4,
    /// King.
    King = 5,
}

impl PieceKind {
    pub(super) const COUNT: usize = 6;
    pub(super) const ALL: [Self; Self::COUNT] = [
        Self::Pawn,
        Self::Knight,
        Self::Bishop,
        Self::Rook,
        Self::Queen,
        Self::King,
    ];

    pub(super) const fn index(self) -> usize {
        self as usize
    }
}

/// A colored chess piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece {
    /// The piece's color.
    pub color: Color,
    /// The piece's kind.
    pub kind: PieceKind,
}

impl Piece {
    /// Creates a piece with the given color and kind.
    #[must_use]
    pub const fn new(color: Color, kind: PieceKind) -> Self {
        Self { color, kind }
    }
}

/// One of the two castling sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CastleSide {
    /// Castling toward the H-file.
    KingSide,
    /// Castling toward the A-file.
    QueenSide,
}

/// The castling permissions retained by a position.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CastlingRights(u8);

impl CastlingRights {
    const WHITE_KINGSIDE: u8 = 1 << 0;
    const WHITE_QUEENSIDE: u8 = 1 << 1;
    const BLACK_KINGSIDE: u8 = 1 << 2;
    const BLACK_QUEENSIDE: u8 = 1 << 3;

    /// No castling rights.
    pub const NONE: Self = Self(0);

    /// Returns whether `color` may castle on `side` according to the stored rights.
    ///
    /// This does not determine whether castling is legal in the current position.
    #[must_use]
    pub const fn allows(self, color: Color, side: CastleSide) -> bool {
        let mask = match (color, side) {
            (Color::White, CastleSide::KingSide) => Self::WHITE_KINGSIDE,
            (Color::White, CastleSide::QueenSide) => Self::WHITE_QUEENSIDE,
            (Color::Black, CastleSide::KingSide) => Self::BLACK_KINGSIDE,
            (Color::Black, CastleSide::QueenSide) => Self::BLACK_QUEENSIDE,
        };

        self.0 & mask != 0
    }

    /// Returns whether no castling rights remain.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(super) fn grant(&mut self, color: Color, side: CastleSide) {
        let mask = match (color, side) {
            (Color::White, CastleSide::KingSide) => Self::WHITE_KINGSIDE,
            (Color::White, CastleSide::QueenSide) => Self::WHITE_QUEENSIDE,
            (Color::Black, CastleSide::KingSide) => Self::BLACK_KINGSIDE,
            (Color::Black, CastleSide::QueenSide) => Self::BLACK_QUEENSIDE,
        };
        self.0 |= mask;
    }
}

/// A square on a chessboard.
///
/// Internally, A1 is index 0 and H8 is index 63.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Square(u8);

impl Square {
    /// Creates a square from zero-based file and rank coordinates.
    #[must_use]
    pub const fn new(file: u8, rank: u8) -> Option<Self> {
        if file < 8 && rank < 8 {
            Some(Self(rank * 8 + file))
        } else {
            None
        }
    }

    /// Creates a square from an index in `0..64`.
    #[must_use]
    pub const fn from_index(index: u8) -> Option<Self> {
        if index < 64 { Some(Self(index)) } else { None }
    }

    /// Returns this square's index in `0..64`.
    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }

    /// Returns the zero-based file, where A is 0 and H is 7.
    #[must_use]
    pub const fn file(self) -> u8 {
        self.0 % 8
    }

    /// Returns the zero-based rank, where rank 1 is 0 and rank 8 is 7.
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0 / 8
    }
}

impl fmt::Display for Square {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = char::from(b'a' + self.file());
        let rank = char::from(b'1' + self.rank());
        write!(formatter, "{file}{rank}")
    }
}

impl FromStr for Square {
    type Err = SquareParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let bytes = value.as_bytes();
        if bytes.len() != 2 {
            return Err(SquareParseError::InvalidLength);
        }

        let file = bytes[0];
        if !(b'a'..=b'h').contains(&file) {
            return Err(SquareParseError::InvalidFile(char::from(file)));
        }

        let rank = bytes[1];
        if !(b'1'..=b'8').contains(&rank) {
            return Err(SquareParseError::InvalidRank(char::from(rank)));
        }

        Self::new(file - b'a', rank - b'1').ok_or(SquareParseError::InvalidLength)
    }
}

/// An error produced while parsing an algebraic square such as `e4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquareParseError {
    /// The input did not contain exactly two ASCII bytes.
    InvalidLength,
    /// The file was not in `a..=h`.
    InvalidFile(char),
    /// The rank was not in `1..=8`.
    InvalidRank(char),
}

impl fmt::Display for SquareParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength => formatter.write_str("a square must contain two ASCII bytes"),
            Self::InvalidFile(file) => write!(formatter, "invalid square file '{file}'"),
            Self::InvalidRank(rank) => write!(formatter, "invalid square rank '{rank}'"),
        }
    }
}

impl Error for SquareParseError {}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::{Color, Square, SquareParseError};

    #[test]
    fn opposite_color_is_an_involution() {
        assert_eq!(Color::White.opposite(), Color::Black);
        assert_eq!(Color::Black.opposite(), Color::White);
    }

    #[test]
    fn every_square_round_trips_through_text() {
        for index in 0..64 {
            let square = Square::from_index(index).expect("index is on the board");
            assert_eq!(Square::from_str(&square.to_string()), Ok(square));
        }
    }

    #[test]
    fn square_parser_rejects_invalid_coordinates() {
        assert_eq!(Square::from_str("e"), Err(SquareParseError::InvalidLength));
        assert_eq!(
            Square::from_str("i4"),
            Err(SquareParseError::InvalidFile('i'))
        );
        assert_eq!(
            Square::from_str("a0"),
            Err(SquareParseError::InvalidRank('0'))
        );
    }
}
