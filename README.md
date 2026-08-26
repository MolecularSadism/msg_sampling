# msg_sampling

Deterministic hashing and sampling primitives for procedural generation in Bevy games.

| Bevy | msg_sampling |
|---|---|
| 0.18 | 0.1.0 |

## What's here

- **Stateless hashing** — `hash1_u32`, `hash1_01`, `tile_hash_u32`, `tile_hash01`.
  Maps `(seed, key, stream)` to a stable value with no RNG state to thread
  through, so a chunk generated in any order yields the same result.
- **Weighted picking** — `pick_weighted`, `pick_weighted_index`. Cumulative-weight
  selection; negative and non-finite weights clamp to zero, non-positive totals
  yield `None`.
- **Disk sampling** — the `DiskSample` extension trait, uniform or eased.
- **Poisson disk sampling** — `generate_poisson_disk_circular`, `PoissonDiskConfig`,
  `SpawnPointSet`, via Bridson's algorithm.

## Bring your own RNG

This crate supplies none. `pick_weighted`, `pick_weighted_index` and `DiskSample`
are bounded on plain `rand::Rng`, so they accept any generator in the `rand`
ecosystem — including [`bevy_rand`](https://crates.io/crates/bevy_rand) /
[`bevy_prng`](https://crates.io/crates/bevy_prng) generators such as `WyRand`,
which is what a Bevy game should reach for when it needs reflected,
serializable, fork-by-seed RNG state.

The hash family and the Poisson sampler take no RNG at all — they are driven
purely by the seed you pass in.

```rust
use msg_sampling::{DiskSample, pick_weighted, tile_hash01};
use rand::SeedableRng;

// No RNG: a stable per-tile decision.
let roll = tile_hash01(0xC0FF_EE, 10, -4, 0);

// With an RNG: anything implementing `rand::Rng`.
let mut rng = rand::rngs::StdRng::seed_from_u64(42);
let loot = pick_weighted(&mut rng, &[("common", 10.0), ("rare", 1.0)]);
let scatter = rng.disk_offset(50.0);
```

## Output stability

The hash family's exact outputs are part of the contract — generated worlds
depend on them, so the mixing constants and structure must not change. The
Poisson sampler is built on that family, so its point sets are equally stable
per seed.

`tests/parity.rs` pins those outputs against values produced by `msg_rng`, this
code's origin, so the extraction is verifiably non-destructive to existing
worlds.

## Relationship to msg_rng

This crate is the RNG-independent half of the retired
[`msg_rng`](https://github.com/MolecularSadism/msg_rng). That crate is
deprecated because it wrapped `rand`'s `StdRng`, which `rand` documents as
non-portable — unsuitable for a crate whose purpose is reproducible seeded
randomness. Its stateful `GlobalRng` / `EntityRng` wrappers are superseded by
`bevy_rand` + `bevy_prng`.

These four modules never depended on those wrappers, so they are carried over
here unchanged rather than retired with them.

## License

MIT OR Apache-2.0.
