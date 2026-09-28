use super::{Color, PieceKind, Position, Square};

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
const KING_OFFSETS: [(i8, i8); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (-1, 1), (-1, -1), (1, -1)];
const ROOK_DIRECTIONS: [(i8, i8); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];

pub(super) fn is_square_attacked(position: &Position, target: Square, by: Color) -> bool {
    let pawn_rank_delta = match by {
        Color::White => 1,
        Color::Black => -1,
    };
    if position
        .bitboard(by, PieceKind::Pawn)
        .squares()
        .any(|from| {
            offset(from, 1, pawn_rank_delta) == Some(target)
                || offset(from, -1, pawn_rank_delta) == Some(target)
        })
    {
        return true;
    }

    if attacks_with_offsets(position, target, by, PieceKind::Knight, &KNIGHT_OFFSETS)
        || attacks_with_offsets(position, target, by, PieceKind::King, &KING_OFFSETS)
    {
        return true;
    }

    attacks_along(
        position,
        target,
        by,
        &[PieceKind::Bishop, PieceKind::Queen],
        &BISHOP_DIRECTIONS,
    ) || attacks_along(
        position,
        target,
        by,
        &[PieceKind::Rook, PieceKind::Queen],
        &ROOK_DIRECTIONS,
    )
}

pub(super) fn offset(square: Square, file_delta: i8, rank_delta: i8) -> Option<Square> {
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

pub(super) const fn knight_offsets() -> &'static [(i8, i8)] {
    &KNIGHT_OFFSETS
}

pub(super) const fn king_offsets() -> &'static [(i8, i8)] {
    &KING_OFFSETS
}

pub(super) const fn bishop_directions() -> &'static [(i8, i8)] {
    &BISHOP_DIRECTIONS
}

pub(super) const fn rook_directions() -> &'static [(i8, i8)] {
    &ROOK_DIRECTIONS
}

fn attacks_with_offsets(
    position: &Position,
    target: Square,
    by: Color,
    kind: PieceKind,
    offsets: &[(i8, i8)],
) -> bool {
    position.bitboard(by, kind).squares().any(|from| {
        offsets
            .iter()
            .any(|&(df, dr)| offset(from, df, dr) == Some(target))
    })
}

fn attacks_along(
    position: &Position,
    target: Square,
    by: Color,
    kinds: &[PieceKind],
    directions: &[(i8, i8)],
) -> bool {
    for &kind in kinds {
        for from in position.bitboard(by, kind).squares() {
            for &(df, dr) in directions {
                let mut current = from;
                while let Some(next) = offset(current, df, dr) {
                    if next == target {
                        return true;
                    }
                    if position.piece_at(next).is_some() {
                        break;
                    }
                    current = next;
                }
            }
        }
    }
    false
}
