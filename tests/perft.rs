//! Perft regression tests for legal move generation.

use chess_engine::chess::{Position, perft};

fn assert_perft(fen: &str, expected: &[u64]) {
    let position = Position::from_fen(fen).expect("fixture FEN is valid");
    for (depth, &nodes) in expected.iter().enumerate() {
        let depth = u8::try_from(depth + 1).expect("fixture depth fits in u8");
        assert_eq!(
            perft(&position, depth),
            Ok(nodes),
            "depth {depth} for {fen}"
        );
    }
}

#[test]
fn starting_position_matches_reference_counts_through_depth_five() {
    assert_eq!(perft(&Position::starting(), 0), Ok(1));
    assert_perft(
        Position::STARTING_FEN,
        &[20, 400, 8_902, 197_281, 4_865_609],
    );
}

#[test]
fn kiwipete_matches_reference_counts_through_depth_four() {
    assert_perft(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &[48, 2_039, 97_862, 4_085_603],
    );
}

#[test]
fn rook_and_pawn_endgame_matches_reference_counts_through_depth_five() {
    assert_perft(
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        &[14, 191, 2_812, 43_238, 674_624],
    );
}

#[test]
fn promotion_and_castling_stress_position_matches_reference_counts() {
    assert_perft(
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        &[6, 264, 9_467, 422_333],
    );
}

#[test]
fn tactical_castling_position_matches_reference_counts() {
    assert_perft(
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        &[44, 1_486, 62_379, 2_103_487],
    );
}

#[test]
fn middlegame_position_matches_reference_counts() {
    assert_perft(
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        &[46, 2_079, 89_890, 3_894_594],
    );
}

#[test]
#[ignore = "deep release-mode milestone verification"]
fn deep_reference_counts_match() {
    let fixtures = [
        (Position::STARTING_FEN, 6, 119_060_324),
        (
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            5,
            193_690_690,
        ),
        (
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            5,
            15_833_292,
        ),
        (
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            5,
            89_941_194,
        ),
        (
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            5,
            164_075_551,
        ),
    ];

    for (fen, depth, expected) in fixtures {
        let position = Position::from_fen(fen).expect("fixture FEN is valid");
        assert_eq!(
            perft(&position, depth),
            Ok(expected),
            "depth {depth} for {fen}"
        );
    }
}
