//! Deterministic hashing and sampling primitives for procedural generation.
//!
//! Four independent pieces, none of which own or wrap an RNG:
//!
//! - **Stateless hashing** — [`hash1_u32`], [`hash1_01`], [`tile_hash_u32`],
//!   [`tile_hash01`]. Maps `(seed, key, stream)` to a stable value with no RNG
//!   state to thread through, so a chunk generated in any order yields the
//!   same result.
//! - **Weighted picking** — [`pick_weighted`], [`pick_weighted_index`].
//! - **Disk sampling** — the [`DiskSample`] extension trait.
//! - **Poisson disk sampling** — [`generate_poisson_disk_circular`],
//!   [`PoissonDiskConfig`], [`SpawnPointSet`], via Bridson's algorithm.
//!
//! # Choosing an RNG
//!
//! This crate deliberately supplies none. [`pick_weighted`],
//! [`pick_weighted_index`] and [`DiskSample`] are bounded on plain
//! [`rand::Rng`], so they accept any generator in the `rand` ecosystem —
//! including [`bevy_rand`](https://crates.io/crates/bevy_rand) /
//! [`bevy_prng`](https://crates.io/crates/bevy_prng) generators such as
//! `WyRand`, which is what Bevy games should reach for when they need
//! reflected, serializable, fork-by-seed RNG state.
//!
//! The hash family and the Poisson sampler take no RNG at all — they are
//! driven purely by the seed you pass in.
//!
//! # Output stability
//!
//! The hash family's exact outputs are part of the contract: generated worlds
//! depend on them, so the mixing constants and structure must not change. The
//! Poisson sampler is built on that family, so its point sets are equally
//! stable per seed.
//!
//! # Example
//!
//! ```rust
//! use msg_sampling::{PoissonDiskConfig, generate_poisson_disk_circular, tile_hash01};
//!
//! // Per-tile decision, no RNG state threaded through.
//! let roll = tile_hash01(0xC0FFEE, 10, -4, 0);
//! assert!((0.0..1.0).contains(&roll));
//!
//! // Evenly spaced spawn points inside a radius.
//! let points = generate_poisson_disk_circular(&PoissonDiskConfig::new(42, 20, 100.0));
//! assert!(!points.is_empty());
//! ```

mod disk;
mod hash;
mod poisson;
mod weighted;

pub use disk::DiskSample;
pub use hash::{hash1_01, hash1_u32, tile_hash_u32, tile_hash01};
pub use poisson::{PoissonDiskConfig, SpawnPointSet, generate_poisson_disk_circular};
pub use weighted::{pick_weighted, pick_weighted_index};

/// Everything you normally want in scope.
pub mod prelude {
    pub use crate::{
        DiskSample, PoissonDiskConfig, SpawnPointSet, generate_poisson_disk_circular, hash1_01,
        hash1_u32, pick_weighted, pick_weighted_index, tile_hash_u32, tile_hash01,
    };
}

#[cfg(test)]
mod test_rng {
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// A fixed generator for tests.
    ///
    /// The tests assert behaviour — bounds, clamping, distribution — never an
    /// exact stream, so the choice of algorithm here is not load-bearing.
    pub fn seeded(seed: u64) -> StdRng {
        StdRng::seed_from_u64(seed)
    }
}
