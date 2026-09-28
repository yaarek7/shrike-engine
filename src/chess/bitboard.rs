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

    /// Iterates over contained squares from least to most significant bit.
    pub fn squares(self) -> impl Iterator<Item = Square> {
        BitboardIter(self.0)
    }

    pub(super) fn insert(&mut self, square: Square) {
        self.0 |= Self::from_square(square).0;
    }

    pub(super) fn remove(&mut self, square: Square) {
        self.0 &= !Self::from_square(square).0;
    }
}

struct BitboardIter(u64);

impl Iterator for BitboardIter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0 == 0 {
            return None;
        }
        let index = self.0.trailing_zeros();
        self.0 &= self.0 - 1;
        Square::from_index(u8::try_from(index).expect("a u64 bit index fits in u8"))
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

    #[test]
    fn square_iteration_is_complete_and_ordered() {
        let board = Bitboard::from_bits((1_u64 << 63) | (1_u64 << 7) | 1);
        let indices: Vec<_> = board.squares().map(Square::index).collect();

        assert_eq!(indices, vec![0, 7, 63]);
    }
}
