//! Criterion benchmarks for the `thing-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator will exercise: single-pair probabilistic matching,
//! deterministic matching, and the batch ranking entry point. Numbers
//! are reported in absolute time per call.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use thing_matcher::{Identifier, MatchConfig, MatchingEngine, SimilarityAlgorithm, Thing};

fn build_eiffel() -> Thing {
    Thing::builder()
        .name("Eiffel Tower")
        .add_alternate_name("La Tour Eiffel")
        .add_alternate_name("Tour Eiffel")
        .description("Iron lattice tower on the Champ de Mars in Paris, France.")
        .url("https://www.toureiffel.paris/")
        .image("https://example.org/images/eiffel.jpg")
        .add_identifier(Identifier::new("wikidata", "Q243").unwrap())
        .add_same_as("https://www.wikidata.org/wiki/Q243")
        .add_same_as("https://en.wikipedia.org/wiki/Eiffel_Tower")
        .add_additional_type("https://schema.org/Landmark")
        .main_entity_of_page("https://en.wikipedia.org/wiki/Eiffel_Tower")
        .build()
}

fn build_eiffel_fuzzy(_seed: &Thing) -> Thing {
    Thing::builder()
        .name("Tour Eiffel")
        .add_alternate_name("Eiffel Tower")
        .description("Iron lattice tower in Paris.")
        .url("https://www.toureiffel.paris/")
        .add_identifier(Identifier::new("wikidata", "Q243").unwrap())
        .add_same_as("https://www.wikidata.org/wiki/Q243")
        .add_additional_type("https://schema.org/Landmark")
        .build()
}

fn build_unrelated() -> Thing {
    Thing::builder()
        .name("Sydney Opera House")
        .description("Multi-venue performing arts centre at Sydney Harbour.")
        .url("https://www.sydneyoperahouse.com/")
        .add_identifier(Identifier::new("wikidata", "Q45178").unwrap())
        .add_additional_type("https://schema.org/PerformingArtsTheater")
        .build()
}

fn make_candidate(idx: usize) -> Thing {
    let names = ["Cafe Central", "Hotel Royal", "The Park", "City Library"];
    Thing::builder()
        .name(names[idx % names.len()])
        .url(format!("https://example.org/items/{idx}"))
        .add_same_as(format!("https://example.org/refs/{}", idx % 50))
        .add_additional_type("https://schema.org/Place")
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
        b.iter(|| engine.match_things(black_box(&eiffel), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_things(black_box(&eiffel), black_box(&fuzzy)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_things(black_box(&eiffel), black_box(&unrelated)));
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
        let candidates: Vec<Thing> = (0..n).map(make_candidate).collect();
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
        b.iter(|| default.match_things(black_box(&eiffel), black_box(&fuzzy)));
    });
    group.bench_function("strict", |b| {
        b.iter(|| strict.match_things(black_box(&eiffel), black_box(&fuzzy)));
    });
    group.bench_function("jaro_winkler", |b| {
        b.iter(|| jw.match_things(black_box(&eiffel), black_box(&fuzzy)));
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
