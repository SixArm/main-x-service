#![warn(clippy::pedantic)]

//! Bridge-path benchmarks: service-side `Thing` → `to_matcher_thing` →
//! `thing_matcher::MatchingEngine::match_things`.
//!
//! Measures the cost of the adapter projection and the end-to-end bridge so
//! any future change to the adapter or the matcher's hot path shows up as a
//! clear regression. Three groups:
//!
//! - `bridge_adapter_only` — `to_matcher_thing` on minimal and rich records.
//! - `bridge_end_to_end`   — adapter + engine call (the realistic dedup path).
//! - `bridge_one_to_many`  — single query vs. 100 candidates.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

use thing_service::matching::adapter::to_matcher_thing;
use thing_service::matching::matcher_lib::{MatchConfig, MatchingEngine};
use thing_service::models::{
    identifier::{IdentifierType, ThingIdentifier},
    thing::Thing,
};

// -------- fixtures -----------------------------------------------------------

/// A name-only fixture representing the cheapest possible record.
fn minimal_thing() -> Thing {
    Thing::new("Test Thing")
}

/// A fully-populated fixture exercising every adapter projection path.
fn rich_thing() -> Thing {
    let mut t = Thing::new("Pride and Prejudice");
    t.alternate_names = vec!["First Impressions".into()];
    t.description = Some("A novel of manners by Jane Austen.".into());
    t.disambiguating_description = Some("Austen, 1813".into());
    t.additional_type = Some("https://schema.org/Book".into());
    t.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    t.images = vec!["https://example.com/cover.jpg".into()];
    t.main_entity_of_page = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    t.same_as = vec![
        "https://www.wikidata.org/wiki/Q170583".into(),
        "https://openlibrary.org/works/OL1394865W".into(),
    ];
    t.identifiers = vec![
        ThingIdentifier::isbn("9780141439518"),
        ThingIdentifier::new(IdentifierType::Custom("OpenLibrary".into()), "OL1394865W"),
    ];
    t
}

/// A matcher engine with the default configuration.
fn engine() -> MatchingEngine {
    MatchingEngine::new(MatchConfig::default())
}

// =============================================================================
// Group 1: adapter projection cost (minimal vs rich record)
// =============================================================================

/// Benchmark the adapter projection alone for minimal and rich records.
fn bench_adapter_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_adapter_only");
    let minimal = minimal_thing();
    let rich = rich_thing();

    group.bench_function("minimal", |b| {
        b.iter(|| {
            let m = to_matcher_thing(black_box(&minimal));
            black_box(m);
        });
    });
    group.bench_function("rich", |b| {
        b.iter(|| {
            let m = to_matcher_thing(black_box(&rich));
            black_box(m);
        });
    });

    group.finish();
}

// =============================================================================
// Group 2: end-to-end bridge — adapter + engine
// =============================================================================

/// Benchmark the full adapter + engine bridge on clone matches.
fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_end_to_end");
    let engine = engine();
    let minimal_a = minimal_thing();
    let minimal_b = minimal_a.clone();
    let rich_a = rich_thing();
    let rich_b = rich_a.clone();

    group.bench_function("minimal_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_things(
                &to_matcher_thing(black_box(&minimal_a)),
                &to_matcher_thing(black_box(&minimal_b)),
            );
            black_box(r);
        });
    });
    group.bench_function("rich_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_things(
                &to_matcher_thing(black_box(&rich_a)),
                &to_matcher_thing(black_box(&rich_b)),
            );
            black_box(r);
        });
    });

    group.finish();
}

// =============================================================================
// Group 3: one-vs-many — the realistic dedup-on-create path
// =============================================================================

/// Benchmark the one-vs-many dedup path across 10/50/100 candidates.
fn bench_one_to_many(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_one_to_many");
    let engine = engine();
    let query = rich_thing();

    // 100 candidates — varied so the matcher can't trivially short-circuit.
    let candidates: Vec<Thing> = (0..100)
        .map(|i| {
            let mut p = rich_thing();
            p.name = format!("Variant{i}");
            p
        })
        .collect();

    for &n in &[10usize, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let qm = to_matcher_thing(black_box(&query));
                let mut best = 0.0_f64;
                for c in candidates.iter().take(n) {
                    let cm = to_matcher_thing(c);
                    let r = engine.match_things(&qm, &cm);
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
