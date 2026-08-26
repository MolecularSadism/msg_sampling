//! Cross-checks that the extracted code reproduces `msg_rng`'s output exactly.
//!
//! The hash family and the Poisson sampler decide the layout of already-generated
//! worlds, so a drift here is silently destructive. The literals below were
//! produced by `msg_rng` at `27effe6` on the `claude/hash-weighted-sampling`
//! branch and are checked in as a fixed reference.

use msg_sampling::{PoissonDiskConfig, generate_poisson_disk_circular, hash1_u32, tile_hash_u32};

#[test]
fn hash1_u32_matches_msg_rng() {
    let got: Vec<u32> = (0..8).map(|i| hash1_u32(0xC0FF_EE00, i, 3)).collect();
    assert_eq!(
        got,
        vec![
            4_144_542_460,
            2_989_854_952,
            1_474_848_845,
            3_438_448_743,
            70_091_614,
            183_200_382,
            429_937_628,
            2_992_968_119,
        ]
    );
}

#[test]
fn tile_hash_u32_matches_msg_rng() {
    let got: Vec<u32> = (-2..2)
        .flat_map(|x| (-2..2).map(move |y| tile_hash_u32(7, x, y, 1)))
        .collect();
    assert_eq!(got.len(), 16);
    assert_eq!(got[0], 3_821_326_469);
    assert_eq!(got[15], 3_021_676_412);
}

#[test]
fn poisson_point_set_matches_msg_rng() {
    let points = generate_poisson_disk_circular(&PoissonDiskConfig::new(42, 100, 100.0));
    assert_eq!(points.len(), 67);
    assert_eq!((points[0].x, points[0].y), (7, -1));
    assert_eq!((points[66].x, points[66].y), (-17, -98));
}
