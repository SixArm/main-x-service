#![warn(clippy::pedantic)]

//! Criterion benchmarks approximating read-side cost: single and batch
//! `Thing` construction (a proxy for row hydration).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use thing_service::models::thing::Thing;

/// Benchmark constructing a single `Thing`.
fn bench_thing_construction(c: &mut Criterion) {
    c.bench_function("thing_construction", |b| {
        b.iter(|| Thing::new(black_box("Test Thing")))
    });
}

/// Benchmark constructing a batch of 100 `Thing`s.
fn bench_thing_batch_construction(c: &mut Criterion) {
    c.bench_function("thing_batch_construction_100", |b| {
        b.iter(|| {
            let things: Vec<Thing> = (0..100)
                .map(|i| Thing::new(&format!("Thing {i}")))
                .collect();
            black_box(things);
        })
    });
}

criterion_group!(benches, bench_thing_construction, bench_thing_batch_construction);
criterion_main!(benches);
