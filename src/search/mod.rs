//! Correctness-first fixed-depth negamax search with alpha-beta pruning.

use std::{cmp::Reverse, error::Error, fmt};

use crate::{
    chess::{
        Move, Position, PositionError, RepetitionKey,
        movegen::{generate_legal_moves_unchecked, is_in_check_unchecked},
    },
    eval::{Evaluator, Score},
};

const MAX_DEPTH: u8 = 64;
const MAX_QUIESCENCE_PLY: u8 = 32;
const INFINITY: i32 = Score::MAX_MAGNITUDE;
const MATE_SCORE: i32 = 900_000;
const MATE_THRESHOLD: i32 = MATE_SCORE - (MAX_DEPTH as i32 + MAX_QUIESCENCE_PLY as i32 + 1);
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
    quiescence_nodes: u64,
    best_move: Option<Move>,
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

    /// Returns the total number of visited nodes, including quiescence nodes and the root.
    #[must_use]
    pub const fn nodes(&self) -> u64 {
        self.nodes
    }

    /// Returns the number of visited quiescence nodes.
    #[must_use]
    pub const fn quiescence_nodes(&self) -> u64 {
        self.quiescence_nodes
    }

    /// Returns a legal root move when one exists.
    ///
    /// When a draw claim is the selected option, this is a deterministic
    /// protocol fallback and the principal variation is empty.
    #[must_use]
    pub const fn best_move(&self) -> Option<Move> {
        self.best_move
    }

    /// Returns the nominal-depth principal variation in root-to-leaf order.
    ///
    /// Quiescence moves are intentionally excluded. This is empty when a draw
    /// claim or automatic draw is selected at the root.
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
    search_with_history(position, evaluator, depth, &[])
}

/// Searches a position with keys for positions preceding the root in the game.
///
/// `prior_position_keys` must be in chronological order and must not include
/// `position` itself. Move clocks are read from `position`; the supplied keys
/// are used only to detect repetition. This keeps repetition history distinct
/// from any future transposition-table state.
///
/// # Errors
///
/// Returns [`SearchError::InvalidDepth`] for depth zero or greater than 64,
/// and [`SearchError::InvalidPosition`] when root validation fails.
pub fn search_with_history(
    position: &Position,
    evaluator: &dyn Evaluator,
    depth: u8,
    prior_position_keys: &[RepetitionKey],
) -> Result<SearchResult, SearchError> {
    if !(1..=MAX_DEPTH).contains(&depth) {
        return Err(SearchError::InvalidDepth(depth));
    }
    position.validate()?;

    let mut working = position.clone();
    let mut context = SearchContext {
        evaluator,
        nodes: 0,
        quiescence_nodes: 0,
        history: prior_position_keys.to_vec(),
    };
    context.history.push(position.repetition_key());
    let result = context.negamax(&mut working, depth, -INFINITY, INFINITY, 0);
    let best_move = result
        .line
        .first()
        .copied()
        .or_else(|| first_legal_move(position));
    Ok(SearchResult {
        depth,
        score: Score::from_centipawns(result.score),
        nodes: context.nodes,
        quiescence_nodes: context.quiescence_nodes,
        best_move,
        principal_variation: result.line,
    })
}

struct SearchContext<'a> {
    evaluator: &'a dyn Evaluator,
    nodes: u64,
    quiescence_nodes: u64,
    history: Vec<RepetitionKey>,
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
        ply: u16,
    ) -> NodeResult {
        if depth == 0 {
            return NodeResult {
                score: self.quiescence(position, alpha, beta, ply, 0),
                line: Vec::new(),
            };
        }
        self.nodes = self.nodes.saturating_add(1);
        let mut moves = generate_legal_moves_unchecked(position);
        if moves.is_empty() {
            let score = if is_in_check_unchecked(position, position.side_to_move()) {
                -MATE_SCORE + i32::from(ply)
            } else {
                0
            };
            return NodeResult {
                score,
                line: Vec::new(),
            };
        }
        let repetitions = self.repetition_count(position.repetition_key());
        if position.halfmove_clock() >= 150
            || repetitions >= 5
            || position.has_insufficient_material()
        {
            return NodeResult {
                score: 0,
                line: Vec::new(),
            };
        }
        let can_claim_draw = position.halfmove_clock() >= 100 || repetitions >= 3;
        order_moves(&mut moves);
        let mut best_score = if can_claim_draw { 0 } else { -INFINITY };
        let mut best_line = Vec::new();
        if can_claim_draw {
            alpha = alpha.max(0);
            if alpha >= beta {
                return NodeResult {
                    score: best_score,
                    line: best_line,
                };
            }
        }
        for chess_move in moves {
            let undo = position.apply_move_unchecked(chess_move);
            self.history.push(position.repetition_key());
            let child = self.negamax(position, depth - 1, -beta, -alpha, ply + 1);
            self.history.pop();
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

    fn quiescence(
        &mut self,
        position: &mut Position,
        mut alpha: i32,
        beta: i32,
        ply: u16,
        quiescence_ply: u8,
    ) -> i32 {
        self.nodes = self.nodes.saturating_add(1);
        self.quiescence_nodes = self.quiescence_nodes.saturating_add(1);

        let mut moves = generate_legal_moves_unchecked(position);
        if moves.is_empty() {
            return if is_in_check_unchecked(position, position.side_to_move()) {
                -MATE_SCORE + i32::from(ply)
            } else {
                0
            };
        }

        let repetitions = self.repetition_count(position.repetition_key());
        if position.halfmove_clock() >= 150
            || repetitions >= 5
            || position.has_insufficient_material()
        {
            return 0;
        }
        let can_claim_draw = position.halfmove_clock() >= 100 || repetitions >= 3;
        let static_evaluation = || {
            let evaluation = self
                .evaluator
                .evaluate(position)
                .centipawns()
                .clamp(-MAX_STATIC_SCORE, MAX_STATIC_SCORE);
            if can_claim_draw {
                evaluation.max(0)
            } else {
                evaluation
            }
        };
        let in_check = is_in_check_unchecked(position, position.side_to_move());
        if quiescence_ply >= MAX_QUIESCENCE_PLY {
            if in_check {
                return self.search_capped_evasions(
                    position,
                    moves,
                    alpha,
                    beta,
                    ply,
                    can_claim_draw,
                );
            }
            return static_evaluation();
        }

        let mut best_score = if in_check {
            if can_claim_draw { 0 } else { -INFINITY }
        } else {
            static_evaluation()
        };
        alpha = alpha.max(best_score);
        if alpha >= beta {
            return best_score;
        }

        if !in_check {
            moves.retain(|chess_move| chess_move.is_capture() || chess_move.promotion().is_some());
        }
        order_moves(&mut moves);
        for chess_move in moves {
            let undo = position.apply_move_unchecked(chess_move);
            self.history.push(position.repetition_key());
            let score = -self.quiescence(position, -beta, -alpha, ply + 1, quiescence_ply + 1);
            self.history.pop();
            position.unmake_move(undo);

            best_score = best_score.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        best_score
    }

    fn search_capped_evasions(
        &mut self,
        position: &mut Position,
        mut moves: Vec<Move>,
        mut alpha: i32,
        beta: i32,
        ply: u16,
        can_claim_draw: bool,
    ) -> i32 {
        order_moves(&mut moves);
        let mut best_score = if can_claim_draw { 0 } else { -INFINITY };
        alpha = alpha.max(best_score);
        if alpha >= beta {
            return best_score;
        }

        for chess_move in moves {
            let undo = position.apply_move_unchecked(chess_move);
            self.history.push(position.repetition_key());
            let score = -self.capped_leaf_score(position, ply + 1);
            self.history.pop();
            position.unmake_move(undo);

            best_score = best_score.max(score);
            alpha = alpha.max(score);
            if alpha >= beta {
                break;
            }
        }
        best_score
    }

    fn capped_leaf_score(&mut self, position: &Position, ply: u16) -> i32 {
        self.nodes = self.nodes.saturating_add(1);
        self.quiescence_nodes = self.quiescence_nodes.saturating_add(1);

        let moves = generate_legal_moves_unchecked(position);
        if moves.is_empty() {
            return if is_in_check_unchecked(position, position.side_to_move()) {
                -MATE_SCORE + i32::from(ply)
            } else {
                0
            };
        }

        let repetitions = self.repetition_count(position.repetition_key());
        if position.halfmove_clock() >= 150
            || repetitions >= 5
            || position.has_insufficient_material()
        {
            return 0;
        }
        let evaluation = self
            .evaluator
            .evaluate(position)
            .centipawns()
            .clamp(-MAX_STATIC_SCORE, MAX_STATIC_SCORE);
        if position.halfmove_clock() >= 100 || repetitions >= 3 {
            evaluation.max(0)
        } else {
            evaluation
        }
    }

    fn repetition_count(&self, current: RepetitionKey) -> usize {
        self.history.iter().filter(|&&key| key == current).count()
    }
}

fn order_moves(moves: &mut [Move]) {
    moves.sort_by_key(|chess_move| Reverse((move_priority(*chess_move), chess_move.raw())));
}

fn first_legal_move(position: &Position) -> Option<Move> {
    let mut moves = generate_legal_moves_unchecked(position);
    order_moves(&mut moves);
    moves.first().copied()
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
    use std::cell::Cell;

    use super::{
        INFINITY, MATE_SCORE, MAX_QUIESCENCE_PLY, MAX_STATIC_SCORE, SearchContext, SearchError,
        search, search_with_history,
    };
    use crate::{
        chess::{
            Color, Move, Piece, PieceKind, Position, Square,
            movegen::generate_legal_moves_unchecked,
        },
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
        assert!(result.quiescence_nodes() > 0);
        assert!(result.nodes() >= result.quiescence_nodes());
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
            quiescence_nodes: 0,
            history: vec![position.repetition_key()],
        };
        let alpha_beta = context.negamax(&mut alpha_beta_position, 2, -INFINITY, INFINITY, 0);
        let mut reference_position = position;
        let mut reference_nodes = 0;
        let reference = unpruned(
            &mut reference_position,
            &ClassicalEvaluator,
            2,
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
        assert!(first.principal_variation().len() <= usize::from(first.depth()));
    }

    #[test]
    fn losing_side_can_claim_third_occurrence_with_a_legal_bestmove() {
        let position = Position::from_fen("4k3/8/8/8/8/8/7Q/4K3 b - - 0 1").expect("valid FEN");
        let prior = [position.repetition_key(), position.repetition_key()];
        let result = search_with_history(&position, &ClassicalEvaluator, 3, &prior)
            .expect("search succeeds");

        assert_eq!(result.score(), Score::ZERO);
        assert!(result.nodes() > 1);
        assert!(result.best_move().is_some());
        assert!(result.principal_variation().is_empty());
    }

    #[test]
    fn second_occurrence_is_not_adjudicated_as_threefold() {
        let position = Position::starting();
        let prior = [position.repetition_key()];
        let result = search_with_history(&position, &ClassicalEvaluator, 1, &prior)
            .expect("search succeeds");

        assert!(result.nodes() > 1);
        assert!(result.best_move().is_some());
    }

    #[test]
    fn losing_side_can_claim_fifty_move_draw_with_a_legal_bestmove() {
        let position = Position::from_fen("4k3/8/8/8/8/8/7Q/4K3 b - - 100 51").expect("valid FEN");
        let result = search(&position, &ClassicalEvaluator, 3).expect("search succeeds");

        assert_eq!(result.score(), Score::ZERO);
        assert!(result.nodes() > 1);
        assert!(result.best_move().is_some());
        assert!(result.principal_variation().is_empty());
    }

    #[test]
    fn winning_continuation_outranks_each_claimable_draw() {
        let fifty_move =
            Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 w - - 100 51").expect("valid FEN");
        let fifty_result = search(&fifty_move, &ClassicalEvaluator, 1).expect("search succeeds");
        let repetition_position =
            Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 w - - 0 1").expect("valid FEN");
        let prior = [
            repetition_position.repetition_key(),
            repetition_position.repetition_key(),
        ];
        let repetition_result =
            search_with_history(&repetition_position, &ClassicalEvaluator, 1, &prior)
                .expect("search succeeds");

        for result in [fifty_result, repetition_result] {
            assert_eq!(result.mate_in(), Some(1));
            assert_eq!(
                result.best_move().map(Move::to_uci).as_deref(),
                Some("f7f8")
            );
        }
    }

    #[test]
    fn automatic_draws_are_distinct_from_claimable_draws() {
        let seventy_five =
            Position::from_fen("4k3/8/8/8/8/8/7Q/4K3 b - - 150 76").expect("valid FEN");
        let seventy_five_result =
            search(&seventy_five, &ClassicalEvaluator, 3).expect("search succeeds");
        let fivefold = Position::from_fen("4k3/8/8/8/8/8/7Q/4K3 b - - 0 1").expect("valid FEN");
        let prior = [fivefold.repetition_key(); 4];
        let fivefold_result = search_with_history(&fivefold, &ClassicalEvaluator, 3, &prior)
            .expect("search succeeds");

        for result in [seventy_five_result, fivefold_result] {
            assert_eq!(result.score(), Score::ZERO);
            assert_eq!(result.nodes(), 1);
            assert!(result.best_move().is_some());
            assert!(result.principal_variation().is_empty());
        }
    }

    #[test]
    fn insufficient_material_is_an_automatic_draw() {
        let position = Position::from_fen("4k3/8/8/8/8/8/8/2B1K3 w - - 0 1").expect("valid FEN");
        let result = search(&position, &ClassicalEvaluator, 3).expect("search succeeds");

        assert_eq!(result.score(), Score::ZERO);
        assert_eq!(result.nodes(), 1);
        assert!(result.best_move().is_some());
        assert!(result.principal_variation().is_empty());
    }

    #[test]
    fn checkmate_precedes_draw_adjudication() {
        let position = Position::from_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 100 51").expect("valid FEN");
        let prior = [position.repetition_key(), position.repetition_key()];
        let result = search_with_history(&position, &ClassicalEvaluator, 3, &prior)
            .expect("search succeeds");

        assert_eq!(result.mate_in(), Some(0));
        assert_ne!(result.score(), Score::ZERO);
    }

    #[test]
    fn quiescence_rejects_a_poisoned_capture() {
        let position = Position::from_fen("4k3/8/5n2/3p4/8/8/8/3QK3 w - - 0 1").expect("valid FEN");
        let result = search(&position, &ClassicalEvaluator, 1).expect("search succeeds");

        assert_ne!(
            result.best_move().map(Move::to_uci).as_deref(),
            Some("d1d5")
        );
        eprintln!(
            "poisoned capture: total nodes {}, quiescence nodes {}",
            result.nodes(),
            result.quiescence_nodes()
        );
        assert!(result.quiescence_nodes() > 0);
        assert_eq!(result.principal_variation().len(), 1);
    }

    #[test]
    fn quiescence_stabilizes_a_forced_recapture() {
        let position = Position::from_fen("4k3/8/5n2/3Q4/8/8/8/4K3 b - - 0 1").expect("valid FEN");
        let stand_pat = ClassicalEvaluator.evaluate(&position).centipawns();
        let (score, nodes) = quiescence_score(&position);

        assert!(score > stand_pat + 500);
        assert!(nodes > 1);
    }

    #[test]
    fn quiescence_searches_quiet_promotions_and_en_passant() {
        for fen in [
            "8/P3k3/8/8/8/8/8/4K3 w - - 0 1",
            "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1",
        ] {
            let position = Position::from_fen(fen).expect("valid FEN");
            let stand_pat = ClassicalEvaluator.evaluate(&position).centipawns();
            let (score, nodes) = quiescence_score(&position);

            assert!(score > stand_pat, "expected tactical improvement: {fen}");
            assert!(nodes > 1, "expected a quiescence move: {fen}");
        }
    }

    #[test]
    fn quiescence_searches_all_evasions_when_in_check() {
        let position = Position::from_fen("4k3/8/8/8/8/8/8/4R1K1 b - - 0 1").expect("valid FEN");
        let legal_moves = position.legal_moves().expect("valid position");
        assert!(
            legal_moves
                .iter()
                .all(|chess_move| !chess_move.is_capture())
        );

        let (_, nodes) = quiescence_score(&position);

        assert!(nodes > 1);
    }

    #[test]
    fn quiescence_recognizes_horizon_mate_and_stalemate() {
        let checkmate = Position::from_fen("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1").expect("valid FEN");
        let stalemate = Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").expect("valid FEN");

        assert_eq!(quiescence_score(&checkmate).0, -MATE_SCORE);
        assert_eq!(quiescence_score(&stalemate).0, 0);
    }

    #[test]
    fn quiescence_guard_bounds_checked_extensions() {
        struct ConstantEvaluator;

        impl Evaluator for ConstantEvaluator {
            fn evaluate(&self, _position: &Position) -> Score {
                Score::from_centipawns(321)
            }
        }

        let position = Position::from_fen("4k3/8/8/8/8/8/8/4R1K1 b - - 0 1").expect("valid FEN");
        let legal_moves = position.legal_moves().expect("valid position");
        let mut working = position.clone();
        let mut context = SearchContext {
            evaluator: &ConstantEvaluator,
            nodes: 0,
            quiescence_nodes: 0,
            history: vec![position.repetition_key()],
        };

        let score = context.quiescence(
            &mut working,
            -INFINITY,
            INFINITY,
            u16::from(MAX_QUIESCENCE_PLY),
            MAX_QUIESCENCE_PLY,
        );

        assert_eq!(score, -321);
        assert_ne!(score, ConstantEvaluator.evaluate(&position).centipawns());
        assert_eq!(context.nodes, 1 + legal_moves.len() as u64);
        assert_eq!(context.quiescence_nodes, context.nodes);
        assert_eq!(working, position);
    }

    #[test]
    fn quiescence_searches_every_capture_underpromotion() {
        struct PromotionObserver {
            seen: Cell<u8>,
        }

        impl Evaluator for PromotionObserver {
            fn evaluate(&self, position: &Position) -> Score {
                let promotion_square = Square::new(0, 7).expect("a8 is on board");
                for (bit, kind) in [
                    (1, PieceKind::Knight),
                    (2, PieceKind::Bishop),
                    (4, PieceKind::Rook),
                    (8, PieceKind::Queen),
                ] {
                    if position.piece_at(promotion_square) == Some(Piece::new(Color::White, kind)) {
                        self.seen.set(self.seen.get() | bit);
                    }
                }
                Score::ZERO
            }
        }

        let position = Position::from_fen("r7/1P2k3/8/7p/8/8/8/4K3 w - - 0 1").expect("valid FEN");
        let observer = PromotionObserver { seen: Cell::new(0) };
        let mut working = position.clone();
        let mut context = SearchContext {
            evaluator: &observer,
            nodes: 0,
            quiescence_nodes: 0,
            history: vec![position.repetition_key()],
        };

        context.quiescence(&mut working, -INFINITY, INFINITY, 0, 0);

        assert_eq!(observer.seen.get(), 0b1111);
        assert_eq!(working, position);
    }

    #[test]
    fn quiescence_alpha_beta_matches_unpruned_reference_and_prunes() {
        let position =
            Position::from_fen("4k3/8/2n2n2/3Q4/2n2n2/8/8/4K3 w - - 0 1").expect("valid FEN");
        let mut working = position.clone();
        let mut context = SearchContext {
            evaluator: &ClassicalEvaluator,
            nodes: 0,
            quiescence_nodes: 0,
            history: vec![position.repetition_key()],
        };
        let alpha = -1_000;
        let beta = 1_000;
        let score = context.quiescence(&mut working, alpha, beta, 0, 0);
        let mut reference = position.clone();
        let mut reference_nodes = 0;
        let reference_score = unpruned_quiescence(
            &mut reference,
            &ClassicalEvaluator,
            0,
            0,
            &mut reference_nodes,
        );

        assert_eq!(score, reference_score);
        assert!(score > alpha && score < beta);
        eprintln!(
            "quiescence alpha-beta nodes: {}, unpruned nodes: {reference_nodes}",
            context.quiescence_nodes
        );
        assert!(context.quiescence_nodes < reference_nodes);
        assert_eq!(working, position);
        assert_eq!(reference, position);
    }

    #[test]
    fn mate_discovered_by_a_quiescence_capture_stays_in_the_mate_band() {
        let position = Position::from_fen("7k/7r/6KQ/8/8/8/8/8 w - - 0 1").expect("valid FEN");
        let (score, nodes) = quiescence_score(&position);
        let result = super::SearchResult {
            depth: 1,
            score: Score::from_centipawns(score),
            nodes,
            quiescence_nodes: nodes,
            best_move: None,
            principal_variation: Vec::new(),
        };

        assert_eq!(score, MATE_SCORE - 1);
        assert_eq!(result.mate_in(), Some(1));
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

    fn quiescence_score(position: &Position) -> (i32, u64) {
        let mut working = position.clone();
        let mut context = SearchContext {
            evaluator: &ClassicalEvaluator,
            nodes: 0,
            quiescence_nodes: 0,
            history: vec![position.repetition_key()],
        };
        let score = context.quiescence(&mut working, -INFINITY, INFINITY, 0, 0);
        assert_eq!(working, *position);
        (score, context.quiescence_nodes)
    }

    fn unpruned_quiescence(
        position: &mut Position,
        evaluator: &dyn Evaluator,
        ply: u16,
        quiescence_ply: u8,
        nodes: &mut u64,
    ) -> i32 {
        *nodes += 1;
        let mut moves = generate_legal_moves_unchecked(position);
        if moves.is_empty() {
            return if super::is_in_check_unchecked(position, position.side_to_move()) {
                -MATE_SCORE + i32::from(ply)
            } else {
                0
            };
        }
        let in_check = super::is_in_check_unchecked(position, position.side_to_move());
        if quiescence_ply >= MAX_QUIESCENCE_PLY && !in_check {
            return evaluator
                .evaluate(position)
                .centipawns()
                .clamp(-MAX_STATIC_SCORE, MAX_STATIC_SCORE);
        }
        if quiescence_ply >= MAX_QUIESCENCE_PLY {
            let mut best = -INFINITY;
            for chess_move in moves {
                let undo = position.apply_move_unchecked(chess_move);
                *nodes += 1;
                let child_moves = generate_legal_moves_unchecked(position);
                let child = if child_moves.is_empty() {
                    if super::is_in_check_unchecked(position, position.side_to_move()) {
                        -MATE_SCORE + i32::from(ply + 1)
                    } else {
                        0
                    }
                } else {
                    evaluator
                        .evaluate(position)
                        .centipawns()
                        .clamp(-MAX_STATIC_SCORE, MAX_STATIC_SCORE)
                };
                best = best.max(-child);
                position.unmake_move(undo);
            }
            return best;
        }
        let mut best = if in_check {
            -INFINITY
        } else {
            evaluator
                .evaluate(position)
                .centipawns()
                .clamp(-MAX_STATIC_SCORE, MAX_STATIC_SCORE)
        };
        if !in_check {
            moves.retain(|chess_move| chess_move.is_capture() || chess_move.promotion().is_some());
        }
        for chess_move in moves {
            let undo = position.apply_move_unchecked(chess_move);
            best = best.max(-unpruned_quiescence(
                position,
                evaluator,
                ply + 1,
                quiescence_ply + 1,
                nodes,
            ));
            position.unmake_move(undo);
        }
        best
    }

    fn unpruned(
        position: &mut Position,
        evaluator: &dyn Evaluator,
        depth: u8,
        ply: u16,
        nodes: &mut u64,
    ) -> i32 {
        if depth == 0 {
            return unpruned_quiescence(position, evaluator, ply, 0, nodes);
        }
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
        let mut best = -INFINITY;
        for chess_move in moves {
            let undo = position.apply_move_unchecked(chess_move);
            best = best.max(-unpruned(position, evaluator, depth - 1, ply + 1, nodes));
            position.unmake_move(undo);
        }
        best
    }
}
