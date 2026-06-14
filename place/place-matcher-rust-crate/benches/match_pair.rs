#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `place-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator will exercise: single-pair probabilistic matching,
//! deterministic matching, and the batch ranking entry point. Numbers
//! are reported in absolute time per call.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use place_matcher::{
    Address, MatchConfig, MatchingEngine, Place, PlaceCategory, PlaceId, PlaceIdScheme,
    SimilarityAlgorithm,
};

fn build_eiffel() -> Place {
    Place::builder()
        .name("Eiffel Tower")
        .add_alternate_name("La Tour Eiffel")
        .add_alternate_name("Tour Eiffel")
        .latitude(48.858_222)
        .longitude(2.294_500)
        .category(PlaceCategory::Monument)
        .country_code_as_iso_3166_1_alpha_2("FR")
        .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap())
        .address(
            Address::new()
                .with_line1("Champ de Mars")
                .with_city("Paris")
                .with_postcode("75007"),
        )
        .phone("+33 8 92 70 12 39")
        .email("contact@example.org")
        .build()
}

fn build_eiffel_fuzzy(_seed: &Place) -> Place {
    Place::builder()
        .name("Tour Eiffel")
        .add_alternate_name("Eiffel Tower")
        .latitude(48.858_3)
        .longitude(2.294_5)
        .category(PlaceCategory::Monument)
        .country_code_as_iso_3166_1_alpha_2("FR")
        .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap())
        .address(
            Address::new()
                .with_line1("Champ de Mars")
                .with_city("Paris")
                .with_postcode("75007"),
        )
        .phone("+33 8 92 70 12 39")
        .email("contact@example.org")
        .build()
}

fn build_unrelated() -> Place {
    Place::builder()
        .name("Sydney Opera House")
        .latitude(-33.856_8)
        .longitude(151.215_3)
        .category(PlaceCategory::Theatre)
        .country_code_as_iso_3166_1_alpha_2("AU")
        .build()
}

fn make_candidate(idx: usize) -> Place {
    let cities = ["London", "Cardiff", "Edinburgh", "Belfast", "Manchester"];
    let names = ["Cafe Central", "Hotel Royal", "The Park", "City Library"];
    let offset = f64::from(u32::try_from(idx).unwrap_or(u32::MAX)) * 1e-4;
    Place::builder()
        .name(names[idx % names.len()])
        .latitude(48.858_222 + offset)
        .longitude(2.294_500 + offset)
        .category(if idx.is_multiple_of(2) {
            PlaceCategory::Cafe
        } else {
            PlaceCategory::Hotel
        })
        .address(
            Address::new()
                .with_line1(format!("{} High Street", idx % 200 + 1))
                .with_city(cities[idx % cities.len()])
                .with_postcode("CF10 1AA"),
        )
        .build()
}

fn bench_match_pair(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_pair");
    let eiffel = build_eiffel();
    let fuzzy = build_eiffel_fuzzy(&eiffel);
    let unrelated = build_unrelated();
    let engine = MatchingEngine::default_config();

    group.bench_function("identical_clone", |b| {
        let clone = eiffel.clone();
        b.iter(|| engine.match_places(black_box(&eiffel), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_places(black_box(&eiffel), black_box(&fuzzy)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_places(black_box(&eiffel), black_box(&unrelated)));
    });
    group.finish();
}

fn bench_deterministic_match(c: &mut Criterion) {
    let eiffel = build_eiffel();
    let clone = eiffel.clone();
    let engine = MatchingEngine::default_config();
    c.bench_function("deterministic_match_identical", |b| {
        b.iter(|| engine.deterministic_match(black_box(&eiffel), black_box(&clone)));
    });
}

fn bench_batch_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("rank_one_to_many");
    let engine = MatchingEngine::default_config();
    let query = build_eiffel();

    for &n in &[10usize, 100, 1000] {
        let candidates: Vec<Place> = (0..n).map(make_candidate).collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &candidates, |b, cands| {
            b.iter(|| engine.rank_one_to_many(black_box(&query), black_box(cands)));
        });
    }
    group.finish();
}

fn bench_engine_configurations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_variants");
    let eiffel = build_eiffel();
    let fuzzy = build_eiffel_fuzzy(&eiffel);

    let default = MatchingEngine::default_config();
    let strict = MatchingEngine::new(MatchConfig::strict());
    let jw = MatchingEngine::new(MatchConfig {
        name_algorithm: SimilarityAlgorithm::JaroWinkler,
        ..MatchConfig::default()
    });

    group.bench_function("default", |b| {
        b.iter(|| default.match_places(black_box(&eiffel), black_box(&fuzzy)));
    });
    group.bench_function("strict", |b| {
        b.iter(|| strict.match_places(black_box(&eiffel), black_box(&fuzzy)));
    });
    group.bench_function("jaro_winkler", |b| {
        b.iter(|| jw.match_places(black_box(&eiffel), black_box(&fuzzy)));
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
