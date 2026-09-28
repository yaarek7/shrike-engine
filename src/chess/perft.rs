use super::{Position, PositionError, movegen::generate_legal_moves_unchecked};

/// Counts legal leaf nodes at `depth` from `position`.
///
/// This correctness oracle intentionally performs no transposition caching.
///
/// # Errors
///
/// Returns [`PositionError`] if the root position is invalid.
pub fn perft(position: &Position, depth: u8) -> Result<u64, PositionError> {
    position.validate()?;
    let mut working = position.clone();
    Ok(perft_unchecked(&mut working, depth))
}

fn perft_unchecked(position: &mut Position, depth: u8) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = generate_legal_moves_unchecked(position);
    if depth == 1 {
        return u64::try_from(moves.len()).expect("move count fits in u64");
    }

    let mut nodes = 0_u64;
    for chess_move in moves {
        let undo = position.apply_move_unchecked(chess_move);
        nodes += perft_unchecked(position, depth - 1);
        position.unmake_move(undo);
    }
    nodes
}
