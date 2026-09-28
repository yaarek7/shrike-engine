//! Correctness-first fixed-depth negamax search with alpha-beta pruning.

use std::{cmp::Reverse, error::Error, fmt};

use crate::{
    chess::{Move, Position, PositionError, movegen::generate_legal_moves_unchecked},
    eval::{Evaluator, Score},
};

const MAX_DEPTH: u8 = 64;
const INFINITY: i32 = Score::MAX_MAGNITUDE;
const MATE_SCORE: i32 = 900_000;
const MATE_THRESHOLD: i32 = MATE_SCORE - 64;
const MAX_STATIC_SCORE: i32 = 100_000;

/// A search setup error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchError {
    /// Search depth must be in `1..=64`.
    InvalidDepth(u8),
    /// The root position is not a valid playable position.
    InvalidPosition(PositionError),
}

impl fmt::Display for SearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDepth(depth) => {
                write!(formatter, "search depth {depth} is outside 1..=64")
            }
            Self::InvalidPosition(error) => write!(formatter, "invalid search position: {error}"),
        }
    }
}

impl Error for SearchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidDepth(_) => None,
            Self::InvalidPosition(error) => Some(error),
        }
    }
}

impl From<PositionError> for SearchError {
    fn from(error: PositionError) -> Self {
        Self::InvalidPosition(error)
    }
}

/// The completed result of a fixed-depth search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    depth: u8,
    score: Score,
    nodes: u64,
    principal_variation: Vec<Move>,
}

impl SearchResult {
    /// Returns the requested search depth.
    #[must_use]
    pub const fn depth(&self) -> u8 {
        self.depth
    }

    /// Returns the root score from the root side-to-move perspective.
    #[must_use]
    pub const fn score(&self) -> Score {
        self.score
    }

    /// Returns the number of visited negamax nodes, including the root.
    #[must_use]
    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Returns the best move, or `None` for checkmate or stalemate.
    #[must_use]
    pub fn best_move(&self) -> Option<Move> {
        self.principal_variation.first().copied()
    }

    /// Returns the principal variation in root-to-leaf order.
    #[must_use]
    pub fn principal_variation(&self) -> &[Move] {
        &self.principal_variation
    }

    /// Returns a signed mate distance in moves when the score is in the mate band.
    #[must_use]
    pub fn mate_in(&self) -> Option<i32> {
        let score = self.score.centipawns();
        if score.abs() < MATE_THRESHOLD {
            return None;
        }
        let plies = MATE_SCORE - score.abs();
        let moves = (plies + 1) / 2;
        Some(if score < 0 { -moves } else { moves })
    }
}

/// Searches a position to an exact depth using negamax and alpha-beta pruning.
///
/// # Errors
///
/// Returns [`SearchError::InvalidDepth`] for depth zero or greater than 64,
/// and [`SearchError::InvalidPosition`] when root validation fails.
pub fn search(
    position: &Position,
    evaluator: &dyn Evaluator,
    depth: u8,
) -> Result<SearchResult, SearchError> {
    if !(1..=MAX_DEPTH).contains(&depth) {
        return Err(SearchError::InvalidDepth(depth));
    }
    position.validate()?;

    let mut working = position.clone();
    let mut context = SearchContext {
        evaluator,
        nodes: 0,
    };
    let result = context.negamax(&mut working, depth, -INFINITY, INFINITY, 0);
    Ok(SearchResult {
        depth,
        score: Score::from_centipawns(result.score),
        nodes: context.nodes,
        principal_variation: result.line,
    })
}

struct SearchContext<'a> {
    evaluator: &'a dyn Evaluator,
    nodes: u64,
}

struct NodeResult {
    score: i32,
    line: Vec<Move>,
}

impl SearchContext<'_> {
    fn negamax(
        &mut self,
        position: &mut Position,
        depth: u8,
        mut alpha: i32,
        beta: i32,
        ply: u8,
    ) -> NodeResult {
        self.nodes = self.nodes.saturating_add(1);
        let mut moves = generate_legal_moves_unchecked(position);
        if moves.is_empty() {
            let score = if position
                .is_in_check(position.side_to_move())
                .expect("descendants of a validated root remain valid")
            {
                -MATE_SCORE + i32::from(ply)
            } else {
                0
            };
            return NodeResult {
                score,
                line: Vec::new(),
            };
        }
        if depth == 0 {
            return NodeResult {
                score: self
                    .evaluator
                    .evaluate(position)
                    .centipawns()
                    .clamp(-MAX_STATIC_SCORE, MAX_STATIC_SCORE),
                line: Vec::new(),
            };
        }

        moves.sort_by_key(|chess_move| Reverse((move_priority(*chess_move), chess_move.raw())));
        let mut best_score = -INFINITY;
        let mut best_line = Vec::new();
        for chess_move in moves {
            let undo = position.apply_move_unchecked(chess_move);
            let child = self.negamax(position, depth - 1, -beta, -alpha, ply + 1);
            let score = -child.score;
            position.unmake_move(undo);

            if score > best_score {
                best_score = score;
                best_line.clear();
                best_line.push(chess_move);
                best_line.extend(child.line);
            }
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        NodeResult {
            score: best_score,
            line: best_line,
        }
    }
}

fn move_priority(chess_move: Move) -> i32 {
    let capture = i32::from(chess_move.is_capture()) * 10_000;
    let promotion = chess_move.promotion().map_or(0, |kind| match kind {
        crate::chess::PieceKind::Knight => 320,
        crate::chess::PieceKind::Bishop => 330,
        crate::chess::PieceKind::Rook => 500,
        crate::chess::PieceKind::Queen => 900,
        crate::chess::PieceKind::Pawn | crate::chess::PieceKind::King => 0,
    });
    capture + promotion
}

#[cfg(test)]
mod tests {
    use super::{INFINITY, MATE_SCORE, MAX_STATIC_SCORE, SearchContext, SearchError, search};
    use crate::{
        chess::{Move, Position, movegen::generate_legal_moves_unchecked},
        eval::{ClassicalEvaluator, Evaluator, Score},
    };

    #[test]
    fn depth_must_be_positive() {
        assert_eq!(
            search(&Position::starting(), &ClassicalEvaluator, 0),
            Err(SearchError::InvalidDepth(0))
        );
    }

    #[test]
    fn search_preserves_root_and_returns_a_legal_move() {
        let position = Position::starting();
        let original = position.clone();
        let result = search(&position, &ClassicalEvaluator, 2).expect("search succeeds");

        assert!(
            position
                .legal_moves()
                .expect("startpos is valid")
                .contains(&result.best_move().expect("startpos has a move"))
        );
        assert_eq!(position, original);
        assert_eq!(result.depth(), 2);
        assert!(result.nodes() > 20);
        assert_eq!(result.principal_variation().len(), 2);
    }

    #[test]
    fn search_selects_an_undefended_queen_capture() {
        let position = Position::from_fen("4k3/8/8/8/8/8/q7/R3K3 w - - 0 1").expect("valid FEN");
        let result = search(&position, &ClassicalEvaluator, 1).expect("search succeeds");

        assert_eq!(
            result.best_move().map(Move::to_uci).as_deref(),
            Some("a1a2")
        );
    }

    #[test]
    fn negamax_sign_is_correct_for_black_to_move() {
        let position = Position::from_fen("r3k3/Q7/8/8/8/8/8/4K3 b - - 0 1").expect("valid FEN");
        let result = search(&position, &ClassicalEvaluator, 1).expect("search succeeds");

        assert_eq!(
            result.best_move().map(Move::to_uci).as_deref(),
            Some("a8a7")
        );
        assert!(result.score().centipawns() > 0);
    }

    #[test]
    fn mate_in_one_outranks_static_evaluation() {
        let position = Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1").expect("valid FEN");
        let result = search(&position, &ClassicalEvaluator, 1).expect("search succeeds");

        assert_eq!(result.mate_in(), Some(1));
        assert!(result.score().centipawns() >= MATE_SCORE - 1);
    }

    #[test]
    fn terminal_positions_distinguish_checkmate_and_stalemate() {
        let checkmate = Position::from_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1").expect("valid FEN");
        let stalemate = Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").expect("valid FEN");
        let mate_result = search(&checkmate, &ClassicalEvaluator, 3).expect("search succeeds");
        let draw_result = search(&stalemate, &ClassicalEvaluator, 3).expect("search succeeds");

        assert_eq!(mate_result.best_move(), None);
        assert_eq!(mate_result.mate_in(), Some(0));
        assert_eq!(draw_result.best_move(), None);
        assert_eq!(draw_result.score().centipawns(), 0);
        assert_eq!(draw_result.mate_in(), None);
    }

    #[test]
    fn alpha_beta_matches_unpruned_negamax_and_visits_fewer_nodes() {
        let position = Position::starting();
        let mut alpha_beta_position = position.clone();
        let mut context = SearchContext {
            evaluator: &ClassicalEvaluator,
            nodes: 0,
        };
        let alpha_beta = context.negamax(&mut alpha_beta_position, 3, -INFINITY, INFINITY, 0);
        let mut reference_position = position;
        let mut reference_nodes = 0;
        let reference = unpruned(
            &mut reference_position,
            &ClassicalEvaluator,
            3,
            0,
            &mut reference_nodes,
        );

        eprintln!(
            "alpha-beta nodes: {}, unpruned nodes: {reference_nodes}",
            context.nodes
        );
        assert_eq!(alpha_beta.score, reference);
        assert!(context.nodes < reference_nodes);
    }

    #[test]
    fn repeated_search_is_deterministic_and_pv_is_replayable() {
        let position = Position::starting();
        let first = search(&position, &ClassicalEvaluator, 3).expect("search succeeds");
        let second = search(&position, &ClassicalEvaluator, 3).expect("search succeeds");

        assert_eq!(first, second);
        let mut replay = position;
        for &chess_move in first.principal_variation() {
            replay.make_move(chess_move).expect("PV move is legal");
        }
    }

    #[test]
    fn extreme_static_scores_are_clamped_below_mate_band() {
        struct ExtremeEvaluator;

        impl Evaluator for ExtremeEvaluator {
            fn evaluate(&self, _position: &Position) -> Score {
                Score::from_centipawns(Score::MAX_MAGNITUDE)
            }
        }

        let result = search(&Position::starting(), &ExtremeEvaluator, 1).expect("search succeeds");

        assert_eq!(result.score().centipawns(), -MAX_STATIC_SCORE);
        assert_eq!(result.mate_in(), None);
    }

    #[test]
    fn invalid_root_is_rejected_without_mutation() {
        let position = Position::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").expect("syntax-valid FEN");
        let original = position.clone();

        assert!(matches!(
            search(&position, &ClassicalEvaluator, 1),
            Err(SearchError::InvalidPosition(_))
        ));
        assert_eq!(position, original);
    }

    fn unpruned(
        position: &mut Position,
        evaluator: &dyn Evaluator,
        depth: u8,
        ply: u8,
        nodes: &mut u64,
    ) -> i32 {
        *nodes += 1;
        let moves = generate_legal_moves_unchecked(position);
        if moves.is_empty() {
            return if position
                .is_in_check(position.side_to_move())
                .expect("position remains valid")
            {
                -MATE_SCORE + i32::from(ply)
            } else {
                0
            };
        }
        if depth == 0 {
            return evaluator
                .evaluate(position)
                .centipawns()
                .clamp(-MAX_STATIC_SCORE, MAX_STATIC_SCORE);
        }

        let mut best = -INFINITY;
        for chess_move in moves {
            let undo = position.apply_move_unchecked(chess_move);
            best = best.max(-unpruned(position, evaluator, depth - 1, ply + 1, nodes));
            position.unmake_move(undo);
        }
        best
    }
}
