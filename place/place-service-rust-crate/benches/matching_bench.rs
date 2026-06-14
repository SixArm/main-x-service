#![warn(clippy::pedantic)]

//! Criterion benchmarks for the matching layer.
//!
//! Covers the individual scoring components (name Jaro-Winkler, geo Haversine
//! decay, Soundex) and the combined scorer, plus a one-to-many batch to model
//! the realistic deduplication path of scoring one query against 100
//! candidates.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use place_service::matching::geo::geo_similarity;
use place_service::matching::name::name_similarity;
use place_service::matching::phonetic::soundex;
use place_service::matching::scoring::{MatchWeights, compute_match};
use place_service::models::address::PostalAddress;
use place_service::models::geo::GeoCoordinates;
use place_service::models::place::Place;
use place_service::models::place_type::PlaceType;

/// Benchmark name similarity on exact, fuzzy, and unrelated name pairs.
fn bench_name_similarity(c: &mut Criterion) {
    c.bench_function("name_similarity_exact", |b| {
        b.iter(|| name_similarity(black_box("Central Park"), black_box("Central Park")));
    });
    c.bench_function("name_similarity_fuzzy", |b| {
        b.iter(|| name_similarity(black_box("Central Park"), black_box("Centrl Park")));
    });
    c.bench_function("name_similarity_different", |b| {
        b.iter(|| name_similarity(black_box("Central Park"), black_box("Golden Gate Bridge")));
    });
}

/// Benchmark geo similarity for close and far coordinate pairs.
fn bench_geo_similarity(c: &mut Criterion) {
    let a = GeoCoordinates::new(40.7829, -73.9654);
    let b = GeoCoordinates::new(48.8584, 2.2945);

    c.bench_function("geo_similarity_close", |bench| {
        let near = GeoCoordinates::new(40.7830, -73.9655);
        bench.iter(|| geo_similarity(black_box(&a), black_box(&near)));
    });
    c.bench_function("geo_similarity_far", |bench| {
        bench.iter(|| geo_similarity(black_box(&a), black_box(&b)));
    });
}

/// Benchmark Soundex encoding on short and long words.
fn bench_soundex(c: &mut Criterion) {
    c.bench_function("soundex_short", |b| b.iter(|| soundex(black_box("Park"))));
    c.bench_function("soundex_long", |b| {
        b.iter(|| soundex(black_box("Springfield")));
    });
}

/// Benchmark the full weighted scorer on two rich, near-duplicate places.
fn bench_full_match(c: &mut Criterion) {
    let mut a = Place::new("Central Park");
    a.place_type = Some(PlaceType::Park);
    a.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
    a.address = Some(PostalAddress {
        street_address: Some("14 E 60th St".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });

    let mut b = Place::new("Centrl Park");
    b.place_type = Some(PlaceType::Park);
    b.geo = Some(GeoCoordinates::new(40.7830, -73.9655));
    b.address = Some(PostalAddress {
        street_address: Some("14 East 60th Street".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });

    let weights = MatchWeights::default();

    c.bench_function("full_place_match", |bench| {
        bench.iter(|| compute_match(black_box(&a), black_box(&b), black_box(&weights)));
    });
}

/// Benchmark scoring one query against 100 candidates (the dedup path).
fn bench_batch_matching(c: &mut Criterion) {
    let target = Place::new("Target Place");
    let candidates: Vec<Place> = (0..100)
        .map(|i| Place::new(&format!("Candidate Place {i}")))
        .collect();
    let weights = MatchWeights::default();

    c.bench_function("batch_match_100_candidates", |b| {
        b.iter(|| {
            for c in &candidates {
                black_box(compute_match(
                    black_box(&target),
                    black_box(c),
                    black_box(&weights),
                ));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_name_similarity,
    bench_geo_similarity,
    bench_soundex,
    bench_full_match,
    bench_batch_matching,
);
criterion_main!(benches);
