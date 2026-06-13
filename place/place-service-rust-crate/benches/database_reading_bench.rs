#![warn(clippy::pedantic)]

//! Criterion benchmarks proxying database-read deserialization cost.
//!
//! Without a live database, these measure the in-memory cost of constructing
//! `Place` records (single and batches of 100) — the work that dominates a
//! read path once rows are fetched.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use place_service::models::place::Place;

/// Benchmark constructing a single `Place`.
fn bench_place_construction(c: &mut Criterion) {
    c.bench_function("place_construction", |b| {
        b.iter(|| Place::new(black_box("Test Place")))
    });
}

/// Benchmark constructing a batch of 100 `Place` records.
fn bench_place_batch_construction(c: &mut Criterion) {
    c.bench_function("place_batch_construction_100", |b| {
        b.iter(|| {
            let places: Vec<Place> = (0..100)
                .map(|i| Place::new(&format!("Place {i}")))
                .collect();
            black_box(places);
        })
    });
}

criterion_group!(
    benches,
    bench_place_construction,
    bench_place_batch_construction
);
criterion_main!(benches);
