#![warn(clippy::pedantic)]

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

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

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

/// Smallest realistic fixture: a name-only place. Isolates the adapter's
/// fixed per-call overhead (no optional fields to project).
fn minimal_place() -> Place {
    Place::new("Test Site")
}

/// Fully-populated fixture exercising every adapter routing branch
/// (alternate name, place type, address, geo+elevation, telephone, GLN, an
/// OSM identifier, capacity) — the worst-case projection cost.
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
        latitude: "40.7829".parse().unwrap(),
        longitude: "-73.9654".parse().unwrap(),
        elevation: Some("10.0".parse().unwrap()),
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

/// Construct a matching engine from the default `MatchConfig` — the same
/// preset the bridge tests pin, so benchmark numbers track the tested path.
fn engine() -> MatchingEngine {
    MatchingEngine::new(MatchConfig::default())
}

// =============================================================================
// Group 1: adapter projection cost (minimal vs rich record)
// =============================================================================

/// Benchmark the `to_matcher_place` projection alone, for a minimal vs. a
/// rich record, to attribute any bridge regression to the adapter rather
/// than the matcher.
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

/// Benchmark the full dedup path — adapter projection plus one
/// `match_places` call — on clone pairs (minimal and rich), the realistic
/// per-comparison cost.
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

/// Benchmark one query scored against 10/50/100 varied candidates — the
/// dedup-on-create fan-out — keeping the best score. Candidate names are
/// perturbed so the matcher cannot trivially short-circuit.
fn bench_one_to_many(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_one_to_many");
    let engine = engine();
    let query = rich_place();

    // 100 candidates — varied so the matcher can't trivially short-circuit.
    let candidates: Vec<Place> = (0..100)
        .map(|i| {
            let mut p = rich_place();
            p.name = format!("Variant{i} Park");
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
