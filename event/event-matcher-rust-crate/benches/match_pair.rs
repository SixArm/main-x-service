#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `event-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator will exercise: single-pair probabilistic matching,
//! deterministic matching, and the batch ranking entry point. Numbers
//! are reported in absolute time per call.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use event_matcher::{
    Address, Event, EventCategory, EventId, EventIdScheme, Location, MatchConfig, MatchingEngine,
    SimilarityAlgorithm,
};
use std::hint::black_box;

/// Build a richly-populated reference event (Glastonbury 2024) used as the
/// "query" side in most benchmarks. Exercises every scored field so the
/// timings reflect a realistic, fully-loaded record.
fn build_glasto() -> Event {
    Event::builder()
        .name("Glastonbury Festival 2024")
        .add_alternate_name("Glasto 2024")
        .add_alternate_name("Glastonbury 2024")
        .start_date("2024-06-26T09:00:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .add_event_id(EventId::new(EventIdScheme::Wikidata, "Q15290").unwrap())
        .organizer("Glastonbury Festivals Ltd")
        .location(
            Location::new()
                .with_venue_name("Worthy Farm")
                .with_address(
                    Address::new()
                        .with_line1("Worthy Farm")
                        .with_city("Pilton")
                        .with_postcode("BA4 4BY"),
                )
                .with_latitude(51.150_3)
                .with_longitude(-2.586_2),
        )
        .url("https://glastonburyfestivals.co.uk/")
        .build()
}

/// Build a near-duplicate of [`build_glasto`] with small perturbations
/// (alias-swapped name, 15-minute time shift, jittered coordinates,
/// "Ltd" vs "Limited") — the worst case for the scorer, since every fuzzy
/// path runs rather than short-circuiting. `_seed` is unused but kept so
/// call-sites read as "fuzzy variant of this event".
fn build_glasto_fuzzy(_seed: &Event) -> Event {
    Event::builder()
        .name("Glasto 2024")
        .add_alternate_name("Glastonbury Festival 2024")
        .start_date("2024-06-26T09:15:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .add_event_id(EventId::new(EventIdScheme::Wikidata, "Q15290").unwrap())
        .organizer("Glastonbury Festivals Limited")
        .location(
            Location::new()
                .with_venue_name("Worthy Farm")
                .with_address(
                    Address::new()
                        .with_line1("Worthy Farm")
                        .with_city("Pilton")
                        .with_postcode("BA4 4BY"),
                )
                .with_latitude(51.150_4)
                .with_longitude(-2.586_1),
        )
        .url("https://glastonburyfestivals.co.uk/")
        .build()
}

/// Build an event that shares nothing with [`build_glasto`], to time the
/// "clearly not a match" path (most fields differ or are absent).
fn build_unrelated() -> Event {
    Event::builder()
        .name("Sydney Opera House Gala")
        .start_date("2025-03-15T20:00:00Z")
        .category(EventCategory::MusicEvent)
        .country_code_as_iso_3166_1_alpha_2("AU")
        .build()
}

/// Build the `idx`-th synthetic candidate for the batch-ranking benchmark.
/// `idx` is folded into the city, name, day, category, and house number so
/// the generated set is varied yet fully deterministic across runs.
fn make_candidate(idx: usize) -> Event {
    let cities = ["London", "Cardiff", "Edinburgh", "Belfast", "Manchester"];
    let names = [
        "Open Mic Night",
        "Conference Day",
        "Comedy Hour",
        "Book Club",
    ];
    let day = (idx % 28) + 1;
    Event::builder()
        .name(names[idx % names.len()])
        .start_date(format!("2024-09-{day:02}T20:00:00Z"))
        .category(if idx.is_multiple_of(2) {
            EventCategory::ComedyEvent
        } else {
            EventCategory::ConferenceEvent
        })
        .location(
            Location::new().with_address(
                Address::new()
                    .with_line1(format!("{} High Street", idx % 200 + 1))
                    .with_city(cities[idx % cities.len()])
                    .with_postcode("CF10 1AA"),
            ),
        )
        .build()
}

/// Benchmark single-pair probabilistic matching across three regimes:
/// identical clone, fuzzy near-match, and unrelated pair.
fn bench_match_pair(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_pair");
    let glasto = build_glasto();
    let fuzzy = build_glasto_fuzzy(&glasto);
    let unrelated = build_unrelated();
    let engine = MatchingEngine::default_config();

    group.bench_function("identical_clone", |b| {
        let clone = glasto.clone();
        b.iter(|| engine.match_events(black_box(&glasto), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_events(black_box(&glasto), black_box(&fuzzy)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_events(black_box(&glasto), black_box(&unrelated)));
    });
    group.finish();
}

/// Benchmark the fast deterministic-match path on an identical pair (the
/// case that short-circuits on a shared event id).
fn bench_deterministic_match(c: &mut Criterion) {
    let glasto = build_glasto();
    let clone = glasto.clone();
    let engine = MatchingEngine::default_config();
    c.bench_function("deterministic_match_identical", |b| {
        b.iter(|| engine.deterministic_match(black_box(&glasto), black_box(&clone)));
    });
}

/// Benchmark [`MatchingEngine::rank_one_to_many`] at 10 / 100 / 1000
/// candidates, reporting per-element throughput to expose scaling.
fn bench_batch_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("rank_one_to_many");
    let engine = MatchingEngine::default_config();
    let query = build_glasto();

    for &n in &[10usize, 100, 1000] {
        let candidates: Vec<Event> = (0..n).map(make_candidate).collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &candidates, |b, cands| {
            b.iter(|| engine.rank_one_to_many(black_box(&query), black_box(cands)));
        });
    }
    group.finish();
}

/// Benchmark the same fuzzy pair under different engine configurations
/// (default, strict, Jaro-Winkler-only) to expose any per-config cost.
fn bench_engine_configurations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_variants");
    let glasto = build_glasto();
    let fuzzy = build_glasto_fuzzy(&glasto);

    let default = MatchingEngine::default_config();
    let strict = MatchingEngine::new(MatchConfig::strict());
    let jw = MatchingEngine::new(MatchConfig {
        name_algorithm: SimilarityAlgorithm::JaroWinkler,
        ..MatchConfig::default()
    });

    group.bench_function("default", |b| {
        b.iter(|| default.match_events(black_box(&glasto), black_box(&fuzzy)));
    });
    group.bench_function("strict", |b| {
        b.iter(|| strict.match_events(black_box(&glasto), black_box(&fuzzy)));
    });
    group.bench_function("jaro_winkler", |b| {
        b.iter(|| jw.match_events(black_box(&glasto), black_box(&fuzzy)));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_match_pair,
    bench_deterministic_match,
    bench_batch_ranking,
    bench_engine_configurations
);
criterion_main!(benches);
