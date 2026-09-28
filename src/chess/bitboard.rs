use super::Square;

/// A set of chessboard squares stored in the bits of a `u64`.
///
/// Bit zero represents A1 and bit 63 represents H8.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitboard(u64);

impl Bitboard {
    /// The empty set of squares.
    pub const EMPTY: Self = Self(0);

    /// Creates a bitboard from its raw bit representation.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Creates a bitboard containing one square.
    #[must_use]
    pub const fn from_square(square: Square) -> Self {
        Self(1_u64 << square.index())
    }

    /// Returns the raw bit representation.
    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Returns whether this bitboard contains `square`.
    #[must_use]
    pub const fn contains(self, square: Square) -> bool {
        self.0 & Self::from_square(square).0 != 0
    }

    /// Returns the number of contained squares.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    pub(super) fn insert(&mut self, square: Square) {
        self.0 |= Self::from_square(square).0;
    }
}

#[cfg(test)]
mod tests {
    use super::Bitboard;
    use crate::chess::Square;

    #[test]
    fn square_mapping_uses_least_significant_bit_for_a1() {
        let a1 = Square::new(0, 0).expect("A1 is on the board");
        let h8 = Square::new(7, 7).expect("H8 is on the board");

        assert_eq!(Bitboard::from_square(a1).bits(), 1);
        assert_eq!(Bitboard::from_square(h8).bits(), 1_u64 << 63);
    }
}
