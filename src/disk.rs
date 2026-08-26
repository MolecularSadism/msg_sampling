//! Uniform random sampling over a disk with optional easing.
//!
//! [`DiskSample`] is blanket-implemented for every [`rand::Rng`], so it works
//! with any RNG from the `rand` ecosystem, including `bevy_rand`/`bevy_prng`
//! generators such as `WyRand`.

use bevy::math::Vec2;
use bevy::math::curve::{Curve, EaseFunction};
use rand::{Rng, RngExt};

/// Extension trait for uniform random sampling over a disk, with optional
/// easing to control density falloff.
///
/// Blanket-implemented for every [`rand::Rng`], so any generator from the
/// `rand` ecosystem gets these methods — including `bevy_rand`/`bevy_prng`
/// generators such as `WyRand`.
///
/// # Examples
///
/// ```rust
/// use msg_sampling::DiskSample;
/// use rand::SeedableRng;
///
/// let mut rng = rand::rngs::StdRng::seed_from_u64(42);
/// let offset = rng.disk_offset(50.0);
/// assert!(offset.length() <= 50.0 + f32::EPSILON);
/// ```
pub trait DiskSample: Rng {
    /// Uniform offset inside a disk of the given radius.
    fn disk_offset(&mut self, radius: f32) -> Vec2 {
        let angle = self.random::<f32>() * std::f32::consts::TAU;
        let dist = self.random::<f32>().sqrt() * radius;
        Vec2::from_angle(angle) * dist
    }

    /// Offset shaped by an easing function to control density falloff.
    ///
    /// The easing curve reshapes the radial CDF:
    /// - `QuadraticIn` - more samples near center (tight cluster)
    /// - `QuadraticOut` - pushes samples outward, denser near the rim
    ///
    /// The eased sample is clamped to `[0, 1]`, so eases that undershoot or
    /// overshoot (`BackIn`, `ElasticOut`, ...) are flattened at the disk's
    /// center or boundary instead of producing out-of-range offsets.
    fn disk_offset_eased(&mut self, radius: f32, ease: EaseFunction) -> Vec2 {
        let angle = self.random::<f32>() * std::f32::consts::TAU;
        let u = self.random::<f32>();
        let dist = ease.sample_clamped(u).clamp(0.0, 1.0).sqrt() * radius;
        Vec2::from_angle(angle) * dist
    }
}

impl<R: Rng + ?Sized> DiskSample for R {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_rng::seeded;

    #[test]
    fn disk_offset_within_radius() {
        let mut rng = rand::rng();
        let radius = 50.0;
        for _ in 0..1000 {
            let offset = rng.disk_offset(radius);
            assert!(
                offset.length() <= radius + f32::EPSILON,
                "offset {offset:?} exceeds radius {radius}"
            );
        }
    }

    #[test]
    fn disk_offset_covers_all_quadrants() {
        let mut rng = rand::rng();
        let radius = 100.0;
        let mut quadrants = [0u32; 4];
        let n = 10_000;
        for _ in 0..n {
            let offset = rng.disk_offset(radius);
            let q = match (offset.x >= 0.0, offset.y >= 0.0) {
                (true, true) => 0,
                (false, true) => 1,
                (false, false) => 2,
                (true, false) => 3,
            };
            quadrants[q] += 1;
        }
        for (i, &count) in quadrants.iter().enumerate() {
            let ratio = count as f32 / n as f32;
            assert!(ratio > 0.20, "quadrant {i} only got {ratio:.2}% of samples");
        }
    }

    #[test]
    fn disk_offset_eased_undershooting_ease_stays_finite() {
        let mut rng = seeded(42);
        let radius = 50.0;
        for _ in 0..1000 {
            let offset = rng.disk_offset_eased(radius, EaseFunction::BackIn);
            assert!(
                offset.x.is_finite() && offset.y.is_finite(),
                "eased offset {offset:?} is not finite"
            );
            assert!(
                offset.length() <= radius + f32::EPSILON,
                "eased offset {offset:?} exceeds radius {radius}"
            );
        }
    }

    #[test]
    fn disk_offset_eased_within_radius() {
        let mut rng = rand::rng();
        let radius = 50.0;
        for _ in 0..1000 {
            let offset = rng.disk_offset_eased(radius, EaseFunction::QuadraticIn);
            assert!(
                offset.length() <= radius + f32::EPSILON,
                "eased offset {offset:?} exceeds radius {radius}"
            );
        }
    }
}
