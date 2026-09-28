//! Replaceable position evaluation interfaces.

mod classical;

use std::{fmt, ops::Neg};

use crate::chess::{Color, Position};

pub use classical::ClassicalEvaluator;

/// A chess evaluation measured in centipawns.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Score(i32);

impl Score {
    /// Largest representable score magnitude, leaving a domain suitable for
    /// future mate scores without approaching integer overflow.
    pub const MAX_MAGNITUDE: i32 = 1_000_000;
    /// A neutral score.
    pub const ZERO: Self = Self(0);

    /// Creates a score from centipawns.
    ///
    /// # Panics
    ///
    /// Panics when the magnitude exceeds [`Self::MAX_MAGNITUDE`].
    #[must_use]
    pub const fn from_centipawns(centipawns: i32) -> Self {
        assert!(
            centipawns >= -Self::MAX_MAGNITUDE && centipawns <= Self::MAX_MAGNITUDE,
            "score exceeds the supported domain"
        );
        Self(centipawns)
    }

    /// Returns the score in centipawns.
    #[must_use]
    pub const fn centipawns(self) -> i32 {
        self.0
    }
}

impl Neg for Score {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl fmt::Display for Score {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} cp", self.0)
    }
}

/// Individually observable components of a classical evaluation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct EvaluationBreakdown {
    material: Score,
    activity: Score,
    mobility: Score,
    pawn_structure: Score,
    king_safety: Score,
}

impl EvaluationBreakdown {
    pub(super) const fn new(
        material: Score,
        activity: Score,
        mobility: Score,
        pawn_structure: Score,
        king_safety: Score,
    ) -> Self {
        Self {
            material,
            activity,
            mobility,
            pawn_structure,
            king_safety,
        }
    }

    /// Returns the material component from White's perspective.
    #[must_use]
    pub const fn material(self) -> Score {
        self.material
    }

    /// Returns the piece-activity component from White's perspective.
    #[must_use]
    pub const fn activity(self) -> Score {
        self.activity
    }

    /// Returns the pseudo-mobility component from White's perspective.
    #[must_use]
    pub const fn mobility(self) -> Score {
        self.mobility
    }

    /// Returns the pawn-structure component from White's perspective.
    #[must_use]
    pub const fn pawn_structure(self) -> Score {
        self.pawn_structure
    }

    /// Returns the king-safety component from White's perspective.
    #[must_use]
    pub const fn king_safety(self) -> Score {
        self.king_safety
    }

    /// Returns the sum of all components from White's perspective.
    #[must_use]
    pub const fn total(self) -> Score {
        Score::from_centipawns(
            self.material.centipawns()
                + self.activity.centipawns()
                + self.mobility.centipawns()
                + self.pawn_structure.centipawns()
                + self.king_safety.centipawns(),
        )
    }

    /// Returns the total from `color`'s perspective.
    #[must_use]
    pub fn for_color(self, color: Color) -> Score {
        match color {
            Color::White => self.total(),
            Color::Black => -self.total(),
        }
    }
}

/// A replaceable position evaluator.
pub trait Evaluator {
    /// Evaluates `position` from the side-to-move perspective.
    fn evaluate(&self, position: &Position) -> Score;

    /// Evaluates `position` from `color`'s perspective.
    fn evaluate_for(&self, position: &Position, color: Color) -> Score {
        let score = self.evaluate(position);
        if color == position.side_to_move() {
            score
        } else {
            -score
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Score;

    #[test]
    fn score_domain_boundaries_negate_safely() {
        let maximum = Score::from_centipawns(Score::MAX_MAGNITUDE);
        let minimum = Score::from_centipawns(-Score::MAX_MAGNITUDE);

        assert_eq!(-maximum, minimum);
        assert_eq!(-minimum, maximum);
    }

    #[test]
    #[should_panic(expected = "score exceeds the supported domain")]
    fn score_rejects_values_outside_its_domain() {
        let _ = Score::from_centipawns(Score::MAX_MAGNITUDE + 1);
    }
}
