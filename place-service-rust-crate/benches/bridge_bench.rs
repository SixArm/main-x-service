//! Bridge-path benchmarks: service-side `Place` → `to_matcher_place` →
//! `place_matcher::MatchingEngine::match_places`.
//!
//! Measures the cost of the adapter projection and the end-to-end bridge so
//! any future change to the adapter or the matcher's hot path shows up as a
//! clear regression. Three groups:
//!
//! - `bridge_adapter_only` — `to_matcher_place` on minimal and rich records.
//! - `bridge_end_to_end`   — adapter + engine call (the realistic dedup path).
//! - `bridge_one_to_many`  — single query vs. 100 candidates.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

use place_service::matching::adapter::to_matcher_place;
use place_service::matching::matcher_lib::{MatchConfig, MatchingEngine};
use place_service::models::{
    address::PostalAddress,
    geo::GeoCoordinates,
    identifier::{IdentifierType, PlaceIdentifier},
    place::Place,
    place_type::PlaceType,
};

// -------- fixtures -----------------------------------------------------------

fn minimal_place() -> Place {
    Place::new("Test Site")
}

fn rich_place() -> Place {
    let mut p = Place::new("Central Park");
    p.alternate_name = Some("The Central Park".into());
    p.place_type = Some(PlaceType::Park);
    p.address = Some(PostalAddress {
        street_address: Some("14 E 60th St".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });
    p.geo = Some(GeoCoordinates {
        latitude: 40.7829,
        longitude: -73.9654,
        elevation: Some(10.0),
    });
    p.telephone = Some("+1-212-310-6600".into());
    p.global_location_number = Some("0614141999996".into());
    p.identifiers.push(PlaceIdentifier::new(
        IdentifierType::OpenStreetMap,
        "node:123456789",
    ));
    p.maximum_attendee_capacity = Some(100_000);
    p
}

fn engine() -> MatchingEngine {
    MatchingEngine::new(MatchConfig::default())
}

// =============================================================================
// Group 1: adapter projection cost (minimal vs rich record)
// =============================================================================

fn bench_adapter_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_adapter_only");
    let minimal = minimal_place();
    let rich = rich_place();

    group.bench_function("minimal", |b| {
        b.iter(|| {
            let m = to_matcher_place(black_box(&minimal));
            black_box(m);
        });
    });
    group.bench_function("rich", |b| {
        b.iter(|| {
            let m = to_matcher_place(black_box(&rich));
            black_box(m);
        });
    });

    group.finish();
}

// =============================================================================
// Group 2: end-to-end bridge — adapter + engine
// =============================================================================

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_end_to_end");
    let engine = engine();
    let minimal_a = minimal_place();
    let minimal_b = minimal_a.clone();
    let rich_a = rich_place();
    let rich_b = rich_a.clone();

    group.bench_function("minimal_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_places(
                &to_matcher_place(black_box(&minimal_a)),
                &to_matcher_place(black_box(&minimal_b)),
            );
            black_box(r);
        });
    });
    group.bench_function("rich_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_places(
                &to_matcher_place(black_box(&rich_a)),
                &to_matcher_place(black_box(&rich_b)),
            );
            black_box(r);
        });
    });

    group.finish();
}

// =============================================================================
// Group 3: one-vs-many — the realistic dedup-on-create path
// =============================================================================

fn bench_one_to_many(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_one_to_many");
    let engine = engine();
    let query = rich_place();

    // 100 candidates — varied so the matcher can't trivially short-circuit.
    let candidates: Vec<Place> = (0..100)
        .map(|i| {
            let mut p = rich_place();
            p.name = format!("Variant{} Park", i);
            p
        })
        .collect();

    for &n in &[10usize, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let qm = to_matcher_place(black_box(&query));
                let mut best = 0.0_f64;
                for c in candidates.iter().take(n) {
                    let cm = to_matcher_place(c);
                    let r = engine.match_places(&qm, &cm);
                    if r.score > best {
                        best = r.score;
                    }
                }
                black_box(best);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_adapter_only,
    bench_end_to_end,
    bench_one_to_many,
);
criterion_main!(benches);
