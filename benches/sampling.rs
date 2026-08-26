//! Criterion benchmarks for the stateless hash family, weighted picking, and
//! Poisson disk generation. All inputs are fixed so runs are deterministic.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use msg_sampling::{
    PoissonDiskConfig, generate_poisson_disk_circular, hash1_u32, pick_weighted_index, tile_hash01,
};
use rand::SeedableRng;

fn bench_hash1_u32(c: &mut Criterion) {
    c.bench_function("hash1_u32", |b| {
        b.iter(|| hash1_u32(black_box(42), black_box(7), black_box(1)));
    });
}

fn bench_tile_hash01(c: &mut Criterion) {
    c.bench_function("tile_hash01", |b| {
        b.iter(|| tile_hash01(black_box(42), black_box(7), black_box(-9), black_box(1)));
    });
}

fn bench_pick_weighted_index(c: &mut Criterion) {
    let weights: Vec<f32> = (0..100).map(|i| (i % 10) as f32 + 0.5).collect();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    c.bench_function("pick_weighted_index/100", |b| {
        b.iter(|| pick_weighted_index(&mut rng, black_box(&weights)));
    });
}

fn bench_generate_poisson_disk_circular(c: &mut Criterion) {
    let config = PoissonDiskConfig::new(42, 100, 100.0);
    c.bench_function("generate_poisson_disk_circular/100@r100", |b| {
        b.iter(|| generate_poisson_disk_circular(black_box(&config)));
    });
}

criterion_group!(
    benches,
    bench_hash1_u32,
    bench_tile_hash01,
    bench_pick_weighted_index,
    bench_generate_poisson_disk_circular
);
criterion_main!(benches);
