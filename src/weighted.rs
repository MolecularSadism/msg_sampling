//! Weighted random selection.
//!
//! Cumulative-weight picking over a slice of `(value, weight)` pairs or a
//! parallel slice of weights. Negative and non-finite weights (NaN, infinite)
//! are clamped to zero; a non-positive total yields `None`. Selection is
//! deterministic given the same RNG state.
//!
//! Both functions are bounded on plain [`rand::Rng`], so they work with any
//! generator from the `rand` ecosystem, including `bevy_rand`/`bevy_prng`
//! generators such as `WyRand`.

use rand::{Rng, RngExt};

/// Pick an element from `(value, weight)` pairs with probability proportional
/// to its weight.
///
/// Negative and non-finite (NaN, infinite) weights are treated as zero.
/// Returns `None` when
/// `entries` is empty or the clamped weights sum to a non-positive total.
/// Deterministic given the same RNG state. Accepts any [`rand::Rng`],
/// including `bevy_rand`/`bevy_prng` generators such as `WyRand`.
///
/// # Examples
///
/// ```rust
/// use msg_sampling::pick_weighted;
/// use rand::SeedableRng;
///
/// let mut rng = rand::rngs::StdRng::seed_from_u64(42);
/// let entries = [("common", 10.0), ("rare", 1.0), ("never", 0.0)];
/// let pick = pick_weighted(&mut rng, &entries).unwrap();
/// assert_ne!(*pick, "never");
/// ```
pub fn pick_weighted<'a, R, T>(rng: &mut R, entries: &'a [(T, f32)]) -> Option<&'a T>
where
    R: Rng + ?Sized,
{
    let index = pick_index(rng, entries.iter().map(|(_, w)| *w))?;
    Some(&entries[index].0)
}

/// Pick an index from a parallel slice of weights, with probability
/// proportional to each weight.
///
/// Negative and non-finite (NaN, infinite) weights are treated as zero.
/// Returns `None` when
/// `weights` is empty or the clamped weights sum to a non-positive total.
/// Deterministic given the same RNG state. Accepts any [`rand::Rng`],
/// including `bevy_rand`/`bevy_prng` generators such as `WyRand`.
///
/// # Examples
///
/// ```rust
/// use msg_sampling::pick_weighted_index;
/// use rand::SeedableRng;
///
/// let mut rng = rand::rngs::StdRng::seed_from_u64(42);
/// let weights = [10.0, 1.0, 0.0];
/// let index = pick_weighted_index(&mut rng, &weights).unwrap();
/// assert!(index < 2, "zero-weight index must never be picked");
/// ```
pub fn pick_weighted_index<R>(rng: &mut R, weights: &[f32]) -> Option<usize>
where
    R: Rng + ?Sized,
{
    pick_index(rng, weights.iter().copied())
}

/// Shared cumulative-weight walk over clamped weights.
fn pick_index<R>(rng: &mut R, weights: impl Iterator<Item = f32> + Clone) -> Option<usize>
where
    R: Rng + ?Sized,
{
    // Negatives clamp to 0 via f32::max; non-finite weights (NaN, ±inf) are
    // zeroed as well so the cumulative walk stays well-defined.
    let clamped = weights.map(|w| if w.is_finite() { w.max(0.0) } else { 0.0 });
    let total: f32 = clamped.clone().sum();
    if total <= 0.0 {
        return None;
    }

    let pick = rng.random::<f32>() * total;
    let mut cumulative = 0.0f32;
    let mut last_positive = None;
    for (index, weight) in clamped.enumerate() {
        if weight <= 0.0 {
            continue;
        }
        cumulative += weight;
        if pick < cumulative {
            return Some(index);
        }
        last_positive = Some(index);
    }
    // Floating point rounding can leave `pick` at or past the accumulated
    // total; fall back to the last element that could have been picked.
    last_positive
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rng::seeded;

    #[test]
    fn deterministic_for_same_seed() {
        let entries = [("a", 1.0), ("b", 2.0), ("c", 3.0)];
        let picks1: Vec<&str> = {
            let mut rng = seeded(7);
            (0..20)
                .map(|_| *pick_weighted(&mut rng, &entries).unwrap())
                .collect()
        };
        let picks2: Vec<&str> = {
            let mut rng = seeded(7);
            (0..20)
                .map(|_| *pick_weighted(&mut rng, &entries).unwrap())
                .collect()
        };
        assert_eq!(picks1, picks2);
    }

    #[test]
    fn none_on_empty_or_non_positive_total() {
        let mut rng = seeded(1);
        let empty: [(&str, f32); 0] = [];
        assert!(pick_weighted(&mut rng, &empty).is_none());
        assert!(pick_weighted(&mut rng, &[("a", 0.0), ("b", 0.0)]).is_none());
        assert!(pick_weighted(&mut rng, &[("a", -1.0), ("b", -0.5)]).is_none());
        assert!(pick_weighted_index::<_>(&mut rng, &[]).is_none());
        assert!(pick_weighted_index(&mut rng, &[0.0, -2.0]).is_none());
    }

    #[test]
    fn negative_weights_clamp_to_zero() {
        let mut rng = seeded(2);
        let entries = [("never", -5.0), ("always", 1.0)];
        for _ in 0..100 {
            assert_eq!(*pick_weighted(&mut rng, &entries).unwrap(), "always");
        }
    }

    #[test]
    fn non_finite_weights_are_treated_as_zero() {
        let mut rng = seeded(8);
        let entries = [("real", 1.0), ("inf", f32::INFINITY)];
        for _ in 0..100 {
            assert_eq!(*pick_weighted(&mut rng, &entries).unwrap(), "real");
        }
        assert!(
            pick_weighted_index(&mut rng, &[f32::INFINITY, f32::NEG_INFINITY, f32::NAN]).is_none()
        );
    }

    #[test]
    fn zero_weight_entries_are_never_picked() {
        let mut rng = seeded(3);
        let weights = [0.0, 1.0, 0.0, 2.0, 0.0];
        for _ in 0..200 {
            let index = pick_weighted_index(&mut rng, &weights).unwrap();
            assert!(weights[index] > 0.0, "picked zero-weight index {index}");
        }
    }

    #[test]
    fn heavier_weights_are_picked_more_often() {
        let mut rng = seeded(4);
        let entries = [("light", 1.0), ("heavy", 9.0)];
        let n = 10_000;
        let heavy = (0..n)
            .filter(|_| *pick_weighted(&mut rng, &entries).unwrap() == "heavy")
            .count();
        let ratio = heavy as f32 / n as f32;
        assert!(
            (0.85..0.95).contains(&ratio),
            "expected ~0.9 heavy picks, got {ratio}"
        );
    }

    #[test]
    fn tuple_and_index_variants_agree() {
        let entries = [("a", 0.5), ("b", 2.5), ("c", 1.0)];
        let weights: Vec<f32> = entries.iter().map(|(_, w)| *w).collect();
        let mut rng1 = seeded(5);
        let mut rng2 = seeded(5);
        for _ in 0..50 {
            let picked = *pick_weighted(&mut rng1, &entries).unwrap();
            let index = pick_weighted_index(&mut rng2, &weights).unwrap();
            assert_eq!(picked, entries[index].0);
        }
    }

    #[test]
    fn single_positive_entry_is_always_picked() {
        let mut rng = seeded(6);
        let entries = [("only", 0.001)];
        for _ in 0..50 {
            assert_eq!(*pick_weighted(&mut rng, &entries).unwrap(), "only");
        }
    }
}
