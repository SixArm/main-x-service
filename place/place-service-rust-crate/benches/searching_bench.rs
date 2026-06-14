#![warn(clippy::pedantic)]

//! Criterion benchmarks approximating name-based search.
//!
//! Models a linear scan that scores a query against 100 candidate names via
//! `name_similarity`, for both an exact query and a fuzzy (misspelled) one.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use place_service::matching::name::name_similarity;
use place_service::models::place::Place;

/// Benchmark scoring an exact-match query against 100 names.
fn bench_search_by_name(c: &mut Criterion) {
    let places: Vec<Place> = (0..100)
        .map(|i| Place::new(&format!("Place {i}")))
        .collect();

    c.bench_function("search_by_name_100", |b| {
        b.iter(|| {
            let query = "Place 42";
            for place in &places {
                black_box(name_similarity(black_box(query), black_box(&place.name)));
            }
        });
    });
}

/// Benchmark scoring a fuzzy (misspelled) query against 100 names.
fn bench_search_by_name_fuzzy(c: &mut Criterion) {
    let places: Vec<Place> = (0..100)
        .map(|i| Place::new(&format!("Place {i}")))
        .collect();

    c.bench_function("search_by_name_fuzzy_100", |b| {
        b.iter(|| {
            let query = "Plce 42";
            for place in &places {
                black_box(name_similarity(black_box(query), black_box(&place.name)));
            }
        });
    });
}

criterion_group!(benches, bench_search_by_name, bench_search_by_name_fuzzy);
criterion_main!(benches);
