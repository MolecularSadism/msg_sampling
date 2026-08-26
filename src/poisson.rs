//! Poisson disk sampling utilities for spatial distribution.
//!
//! Implements Bridson's algorithm for fast Poisson disk sampling.
//! Generates evenly-spaced random points with a minimum-spacing guarantee
//! that holds for the continuous sample positions. The returned positions
//! are rounded to integer tiles: exact duplicates produced by rounding are
//! removed, but when the derived spacing is below `sqrt(2)` two points may
//! still land on adjacent tiles closer than the continuous spacing.
//! Suitable for spawning items, enemies, or other entities with spacing constraints.
//!
//! Randomness comes from the stateless [`tile_hash01`] family, so generation
//! is fully deterministic per seed with no RNG state to thread through.
//!

use std::collections::HashSet;

use bevy::math::IVec2;
use bevy::reflect::{Reflect, std_traits::ReflectDefault};

use crate::hash::tile_hash01;

/// Configuration for Poisson disk point generation.
#[derive(Debug, Clone, PartialEq, Reflect)]
#[reflect(Default, PartialEq)]
pub struct PoissonDiskConfig {
    /// Random seed for deterministic generation.
    pub seed: u32,
    /// Target number of points to generate.
    ///
    /// This is a density hint, not an exact count: it determines the derived
    /// point spacing, and the result may undershoot it (the disk fills up
    /// early) or overshoot the internal cap of twice the target by up to one
    /// candidate batch. Targets denser than one point per tile are silently
    /// clamped to the circle's area in tiles.
    pub target_count: u32,
    /// Maximum radius from origin (circular boundary).
    pub radius: f32,
}

impl Default for PoissonDiskConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            target_count: 10,
            radius: 100.0,
        }
    }
}

impl PoissonDiskConfig {
    /// Create a new config with the given seed and target count.
    #[must_use]
    pub fn new(seed: u32, target_count: u32, radius: f32) -> Self {
        Self {
            seed,
            target_count,
            radius,
        }
    }
}

/// Generate evenly-distributed points using Bridson's Poisson disk sampling.
///
/// Bridson's algorithm:
/// 1. Calculate minimum distance based on target count and area
/// 2. Create a background grid for fast neighbor lookup
/// 3. Start with an initial point, add to active list
/// 4. For each active point, generate candidates in annulus (r to 2r)
/// 5. Accept candidates that are far enough from all neighbors
/// 6. Repeat until active list is empty
///
/// Returns integer positions suitable for tile-based placement. The number
/// of points is approximate: see [`PoissonDiskConfig::target_count`] for the
/// undershoot/overshoot and density-clamp rules. The minimum-spacing
/// guarantee holds for the continuous sample positions; rounding to tiles
/// removes exact duplicates, but below a derived spacing of `sqrt(2)` two
/// points may end up on adjacent tiles.
///
/// Randomness comes from the RNG-free [`tile_hash01`](crate::tile_hash01)
/// family, so this function is independent of the crate's deprecated RNG
/// wrappers and carries to a `bevy_rand` + `bevy_prng` world with zero
/// migration cost, bit-identical per seed.
#[must_use]
pub fn generate_poisson_disk_circular(config: &PoissonDiskConfig) -> Vec<IVec2> {
    if config.target_count == 0 || config.radius <= 0.0 {
        return Vec::new();
    }

    // Calculate the minimum distance between points based on target count
    // For a circle: area = π * r², approximate spacing = sqrt(area / count)
    let area = std::f32::consts::PI * config.radius * config.radius;
    let min_distance = (area / config.target_count as f32).sqrt();

    if min_distance < 1.0 {
        // Distance too small, clamp to 1 tile
        return generate_poisson_disk_circular(&PoissonDiskConfig {
            target_count: area as u32,
            ..config.clone()
        });
    }

    // Grid cell size: r/sqrt(2) ensures at most one sample per cell
    let cell_size = min_distance / std::f32::consts::SQRT_2;
    let diameter = config.radius * 2.0;
    let grid_size = (diameter / cell_size).ceil() as usize + 1;

    // Background grid for fast neighbor lookup (-1 means empty)
    let mut grid: Vec<i32> = vec![-1; grid_size * grid_size];

    // Helper to convert world position to grid cell
    let to_grid = |x: f32, y: f32| -> (usize, usize) {
        let gx = ((x + config.radius) / cell_size).floor() as usize;
        let gy = ((y + config.radius) / cell_size).floor() as usize;
        (gx.min(grid_size - 1), gy.min(grid_size - 1))
    };

    // Result points and active list
    let mut points: Vec<(f32, f32)> = Vec::with_capacity(config.target_count as usize);
    let mut active: Vec<usize> = Vec::new();

    // Hash constants for deterministic randomness
    const STREAM_INIT_X: u32 = 0xDEAD_BEEF;
    const STREAM_INIT_Y: u32 = 0xCAFE_BABE;
    const STREAM_ANGLE: u32 = 0x1234_5678;
    const STREAM_RADIUS: u32 = 0x8765_4321;

    // Start with initial point near center (but offset by seed for variation)
    let init_offset = tile_hash01(config.seed, 0, 0, STREAM_INIT_X) * 0.2 - 0.1;
    let init_y_offset = tile_hash01(config.seed, 0, 0, STREAM_INIT_Y) * 0.2 - 0.1;
    let initial_x = init_offset * config.radius;
    let initial_y = init_y_offset * config.radius;

    // Check initial point is within radius
    if initial_x * initial_x + initial_y * initial_y <= config.radius * config.radius {
        let (gx, gy) = to_grid(initial_x, initial_y);
        grid[gy * grid_size + gx] = 0;
        points.push((initial_x, initial_y));
        active.push(0);
    }

    // Number of candidate attempts per active point
    const K: u32 = 30;

    // Process active list
    let mut iteration = 0u32;
    while !active.is_empty() && points.len() < config.target_count as usize * 2 {
        // Pick a random active point
        let active_idx_hash = tile_hash01(config.seed, iteration as i32, 0, 0xABCD_EF01);
        let active_idx = ((active_idx_hash * active.len() as f32) as usize).min(active.len() - 1);
        let point_idx = active[active_idx];
        let (px, py) = points[point_idx];

        let mut found_candidate = false;

        // Try K candidates around this point
        for k in 0..K {
            iteration = iteration.wrapping_add(1);

            // Generate random point in annulus [min_distance, 2*min_distance]
            let angle_hash = tile_hash01(config.seed, iteration as i32, k as i32, STREAM_ANGLE);
            let radius_hash = tile_hash01(config.seed, iteration as i32, k as i32, STREAM_RADIUS);

            let angle = angle_hash * std::f32::consts::TAU;
            let r = min_distance * (1.0 + radius_hash); // [min_distance, 2*min_distance]

            let cx = px + r * angle.cos();
            let cy = py + r * angle.sin();

            // Check if candidate is within circular bounds
            if cx * cx + cy * cy > config.radius * config.radius {
                continue;
            }

            // Check if candidate is far enough from all neighbors
            let (gx, gy) = to_grid(cx, cy);
            let mut valid = true;

            // Check 5x5 neighborhood in grid
            'neighbor_check: for dy in 0..5 {
                for dx in 0..5 {
                    let nx = gx as i32 + dx - 2;
                    let ny = gy as i32 + dy - 2;

                    if nx < 0 || ny < 0 || nx >= grid_size as i32 || ny >= grid_size as i32 {
                        continue;
                    }

                    let neighbor_idx = grid[ny as usize * grid_size + nx as usize];
                    if neighbor_idx >= 0 {
                        let (nx_pos, ny_pos) = points[neighbor_idx as usize];
                        let dist_sq = (cx - nx_pos).powi(2) + (cy - ny_pos).powi(2);
                        if dist_sq < min_distance * min_distance {
                            valid = false;
                            break 'neighbor_check;
                        }
                    }
                }
            }

            if valid {
                // Accept candidate
                let new_idx = points.len();
                grid[gy * grid_size + gx] = new_idx as i32;
                points.push((cx, cy));
                active.push(new_idx);
                found_candidate = true;
            }
        }

        // If no candidate found, remove from active list
        if !found_candidate {
            active.swap_remove(active_idx);
        }
    }

    // Convert to integer positions, dropping the exact duplicates rounding
    // can introduce when min_distance is below sqrt(2). Order is preserved.
    let mut seen = HashSet::with_capacity(points.len());
    points
        .into_iter()
        .map(|(x, y)| IVec2::new(x.round() as i32, y.round() as i32))
        .filter(|p| seen.insert(*p))
        .collect()
}

/// A set of pre-generated spawn points, queryable by region.
///
/// Region queries are a linear scan over the stored points.
#[derive(Debug, Clone, Default, PartialEq, Reflect)]
#[reflect(Default, PartialEq)]
pub struct SpawnPointSet {
    points: Vec<IVec2>,
}

impl SpawnPointSet {
    /// Create a new spawn point set from generated points.
    #[must_use]
    pub fn new(points: Vec<IVec2>) -> Self {
        Self { points }
    }

    /// Generate a spawn point set using Poisson disk sampling within a circular area.
    #[must_use]
    pub fn from_poisson_disk(config: &PoissonDiskConfig) -> Self {
        let points = generate_poisson_disk_circular(config);
        Self::new(points)
    }

    /// All spawn points in the set.
    #[must_use]
    pub fn points(&self) -> &[IVec2] {
        &self.points
    }

    /// Iterate over all spawn points.
    pub fn iter(&self) -> impl Iterator<Item = IVec2> + '_ {
        self.points.iter().copied()
    }

    /// Get all spawn points within a rectangular region.
    pub fn points_in_rect(&self, min: IVec2, max: IVec2) -> impl Iterator<Item = &IVec2> {
        self.points
            .iter()
            .filter(move |p| p.x >= min.x && p.x <= max.x && p.y >= min.y && p.y <= max.y)
    }

    /// Get the number of spawn points.
    #[must_use]
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Whether the set contains no spawn points.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisson_disk_generates_points() {
        let config = PoissonDiskConfig::new(42, 20, 50.0);
        let points = generate_poisson_disk_circular(&config);

        // Should generate some points
        assert!(!points.is_empty());
        // Poisson disk sampling may generate more or fewer than target
        assert!(points.len() >= 5);
        assert!(points.len() <= 100);
    }

    #[test]
    fn poisson_disk_points_within_radius() {
        let config = PoissonDiskConfig::new(123, 50, 100.0);
        let points = generate_poisson_disk_circular(&config);

        let radius_sq = 100.0 * 100.0;
        for point in &points {
            let dist_sq = (point.x * point.x + point.y * point.y) as f32;
            assert!(
                dist_sq <= radius_sq * 1.01, // Small tolerance for rounding
                "Point {:?} outside radius (dist_sq={}, max={})",
                point,
                dist_sq,
                radius_sq
            );
        }
    }

    #[test]
    fn poisson_disk_minimum_spacing() {
        let config = PoissonDiskConfig::new(456, 30, 100.0);
        let points = generate_poisson_disk_circular(&config);

        // Calculate expected minimum distance
        let area = std::f32::consts::PI * 100.0 * 100.0;
        let expected_min_dist = (area / 30.0).sqrt();

        // Check that no two points are too close
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                let dx = (points[i].x - points[j].x) as f32;
                let dy = (points[i].y - points[j].y) as f32;
                let dist = (dx * dx + dy * dy).sqrt();

                // Allow some tolerance due to rounding to integers
                assert!(
                    dist >= expected_min_dist * 0.9 - 2.0,
                    "Points {:?} and {:?} too close: dist={}, min={}",
                    points[i],
                    points[j],
                    dist,
                    expected_min_dist
                );
            }
        }
    }

    #[test]
    fn dense_config_emits_distinct_tile_positions() {
        // Dense enough that min_distance clamps to 1 tile, where rounding
        // would otherwise collapse nearby samples onto the same tile.
        let config = PoissonDiskConfig::new(7, 500, 10.0);
        let points = generate_poisson_disk_circular(&config);
        assert!(!points.is_empty());

        let unique: std::collections::HashSet<_> = points.iter().copied().collect();
        assert_eq!(
            unique.len(),
            points.len(),
            "duplicate tile positions emitted"
        );
    }

    /// Seed 2_996_718 hashes the very first active-list pick
    /// (`tile_hash_u32(seed, 0, 0, 0xABCD_EF01)` = 4_294_967_218) into the
    /// top 128 u32 values, the ones a plain f32 division would round up to
    /// exactly 1.0. Both the unit-mapping clamp and the active-index clamp
    /// must hold for this seed to generate instead of indexing past the
    /// active list.
    #[test]
    fn top_end_active_pick_hash_does_not_panic() {
        let config = PoissonDiskConfig::new(2_996_718, 30, 50.0);
        let points = generate_poisson_disk_circular(&config);
        assert!(!points.is_empty());
    }

    #[test]
    fn poisson_disk_deterministic() {
        let config = PoissonDiskConfig::new(999, 30, 75.0);
        let points1 = generate_poisson_disk_circular(&config);
        let points2 = generate_poisson_disk_circular(&config);

        assert_eq!(points1, points2, "Same seed should produce same points");
    }

    #[test]
    fn poisson_disk_different_seeds_different_points() {
        let config1 = PoissonDiskConfig::new(111, 30, 75.0);
        let config2 = PoissonDiskConfig::new(222, 30, 75.0);
        let points1 = generate_poisson_disk_circular(&config1);
        let points2 = generate_poisson_disk_circular(&config2);

        assert_ne!(
            points1, points2,
            "Different seeds should produce different points"
        );
    }

    #[test]
    fn spawn_point_set_points_in_rect() {
        let points = vec![
            IVec2::new(0, 0),
            IVec2::new(5, 5),
            IVec2::new(10, 10),
            IVec2::new(15, 15),
            IVec2::new(-5, -5),
        ];
        let set = SpawnPointSet::new(points);

        let in_rect: Vec<_> = set
            .points_in_rect(IVec2::new(0, 0), IVec2::new(10, 10))
            .copied()
            .collect();

        assert_eq!(in_rect.len(), 3);
        assert!(in_rect.contains(&IVec2::new(0, 0)));
        assert!(in_rect.contains(&IVec2::new(5, 5)));
        assert!(in_rect.contains(&IVec2::new(10, 10)));
    }

    #[test]
    fn spawn_point_set_exposes_all_points() {
        let points = vec![IVec2::new(1, 2), IVec2::new(-3, 4)];
        let set = SpawnPointSet::new(points.clone());

        assert_eq!(set.points(), points.as_slice());
        let iterated: Vec<IVec2> = set.iter().collect();
        assert_eq!(iterated, points);
    }

    #[test]
    fn empty_config_produces_empty_result() {
        let config = PoissonDiskConfig::new(0, 0, 100.0);
        let points = generate_poisson_disk_circular(&config);
        assert!(points.is_empty());

        let config = PoissonDiskConfig::new(0, 10, 0.0);
        let points = generate_poisson_disk_circular(&config);
        assert!(points.is_empty());
    }
}
