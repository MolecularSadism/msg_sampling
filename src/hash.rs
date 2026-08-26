//! Stateless, deterministic 1D/2D hashing for generation randomness keyed on
//! world data.
//!
//! Maps `(seed, key, stream)` to a stable `u32` with no RNG state to thread
//! through, so a chunk generated in any order yields the same result. `stream`
//! separates independent decisions that share a key — material and variant
//! rolls for the same tile, say. [`tile_hash_u32`] is the per-tile 2D
//! companion, keyed on global tile coordinates.
//!
//! The outputs of this family are stable across versions: generated worlds
//! depend on the exact values, so the mixing constants and structure must not
//! change.
//!
//! This module is pure integer mixing: it holds no RNG state and depends on no
//! RNG trait.
//!
//! # Bridging from `u64` seeds
//!
//! Stateful RNGs generally use `u64` seeds while this family takes `u32`. The
//! recommended bridge folds the high half into the low half so both halves
//! contribute entropy:
//!
//! ```rust
//! let world_seed: u64 = 0xDEAD_BEEF_1234_5678;
//! let hash_seed = (world_seed ^ (world_seed >> 32)) as u32;
//! let roll = msg_sampling::tile_hash01(hash_seed, 10, -4, 0);
//! assert!((0.0..1.0).contains(&roll));
//! ```

/// Largest `f32` strictly below 1.0.
const MAX_BELOW_ONE: f32 = 1.0 - f32::EPSILON / 2.0;

/// Map a hash output onto [0,1).
///
/// Divides by 2^32 and clamps the handful of top-end `u32` values whose
/// quotient would otherwise round up to exactly 1.0.
#[inline]
fn unit_from_hash(h: u32) -> f32 {
    ((h as f32) / 4_294_967_296.0).min(MAX_BELOW_ONE)
}

#[inline]
fn fmix32(mut z: u32) -> u32 {
    // MurmurHash3 32-bit finalizer
    z ^= z >> 16;
    z = z.wrapping_mul(0x85eb_ca6b);
    z ^= z >> 13;
    z = z.wrapping_mul(0xc2b2_ae35);
    z ^= z >> 16;
    z
}

/// 1D stateless hash for categories/materials: mixes `(seed, key, stream)` into a u32.
#[inline]
#[must_use]
pub fn hash1_u32(seed: u32, key: u32, stream: u32) -> u32 {
    let z = seed ^ key.wrapping_mul(0x9E37_79B9) ^ stream.wrapping_mul(0x85EB_CA6B);
    fmix32(z)
}

/// 2D stateless hash for tiles: mixes `(seed, x, y, stream)` into a u32.
#[inline]
#[must_use]
pub fn tile_hash_u32(seed: u32, x: i32, y: i32, stream: u32) -> u32 {
    // Use distinct odd 32-bit constants for each dimension and stream
    let z = seed
        ^ (x as u32).wrapping_mul(0x9E37_79B9)
        ^ (y as u32).wrapping_mul(0xC2B2_AE35)
        ^ stream.wrapping_mul(0x85EB_CA6B);
    fmix32(z)
}

/// 2D stateless hash mapped to [0,1). The upper bound is exclusive: the few
/// `u32` hash values whose `f32` quotient would round up to 1.0 clamp to the
/// largest `f32` below it.
#[inline]
#[must_use]
pub fn tile_hash01(seed: u32, x: i32, y: i32, stream: u32) -> f32 {
    unit_from_hash(tile_hash_u32(seed, x, y, stream))
}

/// 1D stateless hash mapped to [0,1). The upper bound is exclusive: the few
/// `u32` hash values whose `f32` quotient would round up to 1.0 clamp to the
/// largest `f32` below it.
#[inline]
#[must_use]
pub fn hash1_01(seed: u32, key: u32, stream: u32) -> f32 {
    unit_from_hash(hash1_u32(seed, key, stream))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_inputs() {
        assert_eq!(hash1_u32(1, 2, 3), hash1_u32(1, 2, 3));
        assert_eq!(tile_hash_u32(1, -2, 3, 4), tile_hash_u32(1, -2, 3, 4));
    }

    #[test]
    fn differs_across_keys() {
        assert_ne!(hash1_u32(1, 2, 3), hash1_u32(1, 4, 3));
        assert_ne!(tile_hash_u32(1, 2, 3, 4), tile_hash_u32(1, 2, 5, 4));
    }

    #[test]
    fn differs_across_streams() {
        assert_ne!(hash1_u32(1, 2, 3), hash1_u32(1, 2, 4));
        assert_ne!(tile_hash_u32(1, 2, 3, 4), tile_hash_u32(1, 2, 3, 5));
    }

    /// Existing worlds depend on these exact outputs; if this test fails the
    /// hash family is no longer bit-identical and must not be released.
    #[test]
    fn hash1_u32_pinned_reference_values() {
        assert_eq!(hash1_u32(0, 0, 0), 0);
        assert_eq!(hash1_u32(1, 2, 3), 1_034_830_474);
        assert_eq!(hash1_u32(42, 7, 1), 3_747_008_324);
        assert_eq!(hash1_u32(0xDEAD_BEEF, 123, 456), 685_672_292);
    }

    /// Existing worlds depend on these exact outputs; if this test fails the
    /// hash family is no longer bit-identical and must not be released.
    #[test]
    fn tile_hash_u32_pinned_reference_values() {
        assert_eq!(tile_hash_u32(0, 0, 0, 0), 0);
        assert_eq!(tile_hash_u32(1, -3, 4, 2), 3_173_395_918);
        assert_eq!(tile_hash_u32(42, 7, -9, 1), 838_684_485);
        assert_eq!(tile_hash_u32(0xDEAD_BEEF, 123, -456, 7), 2_852_716_746);
    }

    #[test]
    fn unit_range_variants_match_u32_and_stay_in_bounds() {
        assert_eq!(
            tile_hash01(42, 7, -9, 1),
            838_684_485u32 as f32 / 4_294_967_296.0
        );
        assert_eq!(
            hash1_01(42, 7, 1),
            3_747_008_324u32 as f32 / 4_294_967_296.0
        );

        for i in 0..100 {
            let v = tile_hash01(9, i, -i, 3);
            assert!((0.0..1.0).contains(&v), "tile_hash01 out of range: {v}");
            let v = hash1_01(9, i as u32, 3);
            assert!((0.0..1.0).contains(&v), "hash1_01 out of range: {v}");
        }
    }

    /// The 128 largest u32 hash values would round up to exactly 1.0 when
    /// divided as f32; the mapping must clamp them below 1.0 while leaving
    /// every other value bit-identical to the plain division.
    #[test]
    fn unit_from_hash_stays_below_one() {
        assert!(unit_from_hash(u32::MAX) < 1.0);
        assert_eq!(unit_from_hash(u32::MAX), MAX_BELOW_ONE);
        assert_eq!(unit_from_hash(4_294_967_168), MAX_BELOW_ONE);
        assert_eq!(
            unit_from_hash(2_147_483_648),
            2_147_483_648u32 as f32 / 4_294_967_296.0
        );
        assert_eq!(unit_from_hash(0), 0.0);
    }
}
