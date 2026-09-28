use crate::chess::{Color, PieceKind, Position, Square};

use super::{EvaluationBreakdown, Evaluator, Score};

const KNIGHT_OFFSETS: [(i8, i8); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (-1, 1), (-1, -1), (1, -1)];
const ROOK_DIRECTIONS: [(i8, i8); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

/// A deterministic classical evaluator composed of independently observable terms.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ClassicalEvaluator;

impl ClassicalEvaluator {
    /// Returns independently observable terms from White's perspective.
    #[must_use]
    pub fn breakdown(self, position: &Position) -> EvaluationBreakdown {
        EvaluationBreakdown::new(
            difference(position, material),
            difference(position, activity),
            difference(position, mobility),
            difference(position, pawn_structure),
            difference(position, king_safety),
        )
    }
}

impl Evaluator for ClassicalEvaluator {
    fn evaluate(&self, position: &Position) -> Score {
        self.breakdown(position).for_color(position.side_to_move())
    }
}

fn difference(position: &Position, component: fn(&Position, Color) -> i32) -> Score {
    Score::from_centipawns(component(position, Color::White) - component(position, Color::Black))
}

fn material(position: &Position, color: Color) -> i32 {
    [
        (PieceKind::Pawn, 100),
        (PieceKind::Knight, 320),
        (PieceKind::Bishop, 330),
        (PieceKind::Rook, 500),
        (PieceKind::Queen, 900),
        (PieceKind::King, 0),
    ]
    .into_iter()
    .map(|(kind, value)| {
        i32::try_from(position.bitboard(color, kind).count()).expect("piece count fits in i32")
            * value
    })
    .sum()
}

fn activity(position: &Position, color: Color) -> i32 {
    let mut score = 0;
    for kind in [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
    ] {
        for square in position.bitboard(color, kind).squares() {
            let advancement = match color {
                Color::White => i32::from(square.rank()),
                Color::Black => i32::from(7 - square.rank()),
            };
            let center = 6 - center_distance(square);
            score += match kind {
                PieceKind::Pawn => advancement * 5 + center * 2,
                PieceKind::Knight => center * 4,
                PieceKind::Bishop => center * 2,
                PieceKind::Rook => i32::from(advancement == 6) * 10,
                PieceKind::Queen => center,
                PieceKind::King => 0,
            };
        }
    }
    score
}

fn mobility(position: &Position, color: Color) -> i32 {
    let mut count = 0;
    count += leaper_mobility(position, color, PieceKind::Knight, &KNIGHT_OFFSETS);
    count += slider_mobility(position, color, PieceKind::Bishop, &BISHOP_DIRECTIONS);
    count += slider_mobility(position, color, PieceKind::Rook, &ROOK_DIRECTIONS);
    count += slider_mobility(position, color, PieceKind::Queen, &BISHOP_DIRECTIONS);
    count += slider_mobility(position, color, PieceKind::Queen, &ROOK_DIRECTIONS);
    count * 2
}

fn pawn_structure(position: &Position, color: Color) -> i32 {
    let pawns = position.bitboard(color, PieceKind::Pawn);
    let enemy_pawns = position.bitboard(color.opposite(), PieceKind::Pawn);
    let mut file_counts = [0_u8; 8];
    for pawn in pawns.squares() {
        file_counts[usize::from(pawn.file())] += 1;
    }

    let doubled_penalty: i32 = file_counts
        .into_iter()
        .map(|count| i32::from(count.saturating_sub(1)) * 15)
        .sum();
    let mut score = -doubled_penalty;
    for pawn in pawns.squares() {
        let file = usize::from(pawn.file());
        let isolated =
            (file == 0 || file_counts[file - 1] == 0) && (file == 7 || file_counts[file + 1] == 0);
        if isolated {
            score -= 10;
        }

        let passed = enemy_pawns.squares().all(|enemy| {
            pawn.file().abs_diff(enemy.file()) > 1
                || match color {
                    Color::White => enemy.rank() <= pawn.rank(),
                    Color::Black => enemy.rank() >= pawn.rank(),
                }
        });
        if passed {
            let advancement = match color {
                Color::White => pawn.rank(),
                Color::Black => 7 - pawn.rank(),
            };
            score += 15 + i32::from(advancement) * 5;
        }
    }
    score
}

fn king_safety(position: &Position, color: Color) -> i32 {
    let Some(king) = position.bitboard(color, PieceKind::King).squares().next() else {
        return 0;
    };
    let forward = match color {
        Color::White => 1,
        Color::Black => -1,
    };
    let shield = [-1, 0, 1]
        .into_iter()
        .filter_map(|file_delta| offset(king, file_delta, forward))
        .filter(|&square| {
            position.piece_at(square) == Some(crate::chess::Piece::new(color, PieceKind::Pawn))
        })
        .count();
    let castled = matches!(king.file(), 2 | 6)
        && match color {
            Color::White => king.rank() == 0,
            Color::Black => king.rank() == 7,
        };
    i32::try_from(shield).expect("shield count fits in i32") * 12 + i32::from(castled) * 20
}

fn leaper_mobility(
    position: &Position,
    color: Color,
    kind: PieceKind,
    offsets: &[(i8, i8)],
) -> i32 {
    let count = position
        .bitboard(color, kind)
        .squares()
        .flat_map(|from| {
            offsets
                .iter()
                .filter_map(move |&(df, dr)| offset(from, df, dr))
        })
        .filter(|&to| {
            position
                .piece_at(to)
                .is_none_or(|piece| piece.color != color)
        })
        .count();
    i32::try_from(count).expect("mobility count fits in i32")
}

fn slider_mobility(
    position: &Position,
    color: Color,
    kind: PieceKind,
    directions: &[(i8, i8)],
) -> i32 {
    let mut count = 0;
    for from in position.bitboard(color, kind).squares() {
        for &(df, dr) in directions {
            let mut current = from;
            while let Some(to) = offset(current, df, dr) {
                match position.piece_at(to) {
                    None => count += 1,
                    Some(piece) if piece.color != color => {
                        count += 1;
                        break;
                    }
                    Some(_) => break,
                }
                current = to;
            }
        }
    }
    count
}

fn center_distance(square: Square) -> i32 {
    let file = i32::from(square.file());
    let rank = i32::from(square.rank());
    (file - 3).abs().min((file - 4).abs()) + (rank - 3).abs().min((rank - 4).abs())
}

fn offset(square: Square, file_delta: i8, rank_delta: i8) -> Option<Square> {
    let file = i16::from(square.file()) + i16::from(file_delta);
    let rank = i16::from(square.rank()) + i16::from(rank_delta);
    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
        return None;
    }
    Square::new(
        u8::try_from(file).expect("checked file fits in u8"),
        u8::try_from(rank).expect("checked rank fits in u8"),
    )
}

#[cfg(test)]
mod tests {
    use super::ClassicalEvaluator;
    use crate::{
        chess::{Color, Position},
        eval::Evaluator,
    };

    #[test]
    fn starting_position_is_exactly_balanced() {
        let evaluation = ClassicalEvaluator.breakdown(&Position::starting());

        assert_eq!(evaluation.total().centipawns(), 0);
        assert_eq!(evaluation.material().centipawns(), 0);
        assert_eq!(evaluation.activity().centipawns(), 0);
        assert_eq!(evaluation.mobility().centipawns(), 0);
        assert_eq!(evaluation.pawn_structure().centipawns(), 0);
        assert_eq!(evaluation.king_safety().centipawns(), 0);
    }

    #[test]
    fn material_uses_centipawn_piece_values() {
        let position = Position::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").expect("valid FEN");
        let evaluation = ClassicalEvaluator.breakdown(&position);

        assert_eq!(evaluation.material().centipawns(), 900);
        assert!(evaluation.total().centipawns() > 900);
    }

    #[test]
    fn perspective_conversion_is_antisymmetric() {
        let position = Position::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").expect("valid FEN");
        let white = ClassicalEvaluator.evaluate_for(&position, Color::White);
        let black = ClassicalEvaluator.evaluate_for(&position, Color::Black);

        assert_eq!(white, -black);
    }

    #[test]
    fn evaluator_trait_uses_side_to_move_perspective_without_mutation() {
        let white_to_move =
            Position::from_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1").expect("valid FEN");
        let black_to_move =
            Position::from_fen("4k3/8/8/8/8/8/8/3QK3 b - - 0 1").expect("valid FEN");
        let original = white_to_move.clone();

        let white_score = ClassicalEvaluator.evaluate(&white_to_move);
        let repeated = ClassicalEvaluator.evaluate(&white_to_move);
        let black_score = ClassicalEvaluator.evaluate(&black_to_move);

        assert_eq!(white_score, repeated);
        assert_eq!(white_score, -black_score);
        assert_eq!(white_to_move, original);
    }

    #[test]
    fn every_material_value_is_exact() {
        for (symbol, expected) in [('P', 100), ('N', 320), ('B', 330), ('R', 500), ('Q', 900)] {
            let fen = format!("4k3/8/8/8/8/8/8/{symbol}3K3 w - - 0 1");
            let position = Position::from_fen(&fen).expect("valid FEN");

            assert_eq!(
                ClassicalEvaluator
                    .breakdown(&position)
                    .material()
                    .centipawns(),
                expected,
                "wrong value for {symbol}"
            );
        }
    }

    #[test]
    fn central_knight_has_more_activity_than_corner_knight() {
        let corner = Position::from_fen("4k3/8/8/8/8/8/8/N3K3 w - - 0 1").expect("valid FEN");
        let center = Position::from_fen("4k3/8/8/8/3N4/8/8/4K3 w - - 0 1").expect("valid FEN");

        assert!(
            ClassicalEvaluator.breakdown(&center).activity()
                > ClassicalEvaluator.breakdown(&corner).activity()
        );
        assert!(
            ClassicalEvaluator.breakdown(&center).mobility()
                > ClassicalEvaluator.breakdown(&corner).mobility()
        );
    }

    #[test]
    fn slider_mobility_stops_at_friendly_blockers() {
        let open = Position::from_fen("4k3/8/8/8/8/8/8/R3K3 w - - 0 1").expect("valid FEN");
        let blocked = Position::from_fen("4k3/8/8/8/8/8/P7/R3K3 w - - 0 1").expect("valid FEN");

        assert!(
            ClassicalEvaluator.breakdown(&open).mobility()
                > ClassicalEvaluator.breakdown(&blocked).mobility()
        );
    }

    #[test]
    fn pawn_shield_improves_king_safety() {
        let exposed = Position::from_fen("4k3/8/8/8/8/8/8/6K1 w - - 0 1").expect("valid FEN");
        let shielded = Position::from_fen("4k3/8/8/8/8/8/5PPP/6K1 w - - 0 1").expect("valid FEN");

        assert!(
            ClassicalEvaluator.breakdown(&shielded).king_safety()
                > ClassicalEvaluator.breakdown(&exposed).king_safety()
        );
    }

    #[test]
    fn connected_pawns_outscore_doubled_isolated_pawns() {
        let connected = Position::from_fen("4k3/8/8/8/8/8/2PP4/4K3 w - - 0 1").expect("valid FEN");
        let doubled = Position::from_fen("4k3/8/8/8/8/3P4/3P4/4K3 w - - 0 1").expect("valid FEN");

        assert!(
            ClassicalEvaluator.breakdown(&connected).pawn_structure()
                > ClassicalEvaluator.breakdown(&doubled).pawn_structure()
        );
    }

    #[test]
    fn color_swap_and_vertical_mirror_negate_every_component() {
        let original = Position::from_fen("4k3/8/8/3p4/2P5/8/8/3QK3 w - - 0 1").expect("valid FEN");
        let mirrored = Position::from_fen("3qk3/8/8/2p5/3P4/8/8/4K3 b - - 0 1").expect("valid FEN");
        let original = ClassicalEvaluator.breakdown(&original);
        let mirrored = ClassicalEvaluator.breakdown(&mirrored);

        assert_eq!(original.material(), -mirrored.material());
        assert_eq!(original.activity(), -mirrored.activity());
        assert_eq!(original.mobility(), -mirrored.mobility());
        assert_eq!(original.pawn_structure(), -mirrored.pawn_structure());
        assert_eq!(original.king_safety(), -mirrored.king_safety());
        assert_eq!(original.total(), -mirrored.total());
    }
}
