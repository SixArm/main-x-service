#![warn(clippy::pedantic)]

//! Criterion benchmarks approximating name search: scoring a query against
//! 100 records, for exact and fuzzy queries.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use thing_service::matching::name::name_similarity;
use thing_service::models::thing::Thing;

/// Benchmark scoring an exact-name query against 100 records.
fn bench_search_by_name(c: &mut Criterion) {
    let things: Vec<Thing> = (0..100)
        .map(|i| Thing::new(&format!("Thing {i}")))
        .collect();

    c.bench_function("search_by_name_100", |b| {
        b.iter(|| {
            let query = "Thing 42";
            for thing in &things {
                black_box(name_similarity(black_box(query), black_box(&thing.name)));
            }
        });
    });
}

/// Benchmark scoring a fuzzy (typo) query against 100 records.
fn bench_search_by_name_fuzzy(c: &mut Criterion) {
    let things: Vec<Thing> = (0..100)
        .map(|i| Thing::new(&format!("Thing {i}")))
        .collect();

    c.bench_function("search_by_name_fuzzy_100", |b| {
        b.iter(|| {
            let query = "Thng 42";
            for thing in &things {
                black_box(name_similarity(black_box(query), black_box(&thing.name)));
            }
        });
    });
}

criterion_group!(benches, bench_search_by_name, bench_search_by_name_fuzzy);
criterion_main!(benches);
