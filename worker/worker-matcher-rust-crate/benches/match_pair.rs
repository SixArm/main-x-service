#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `worker-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator will exercise: single-pair probabilistic matching,
//! deterministic matching, and the batch ranking entry point. Numbers
//! are reported in absolute time per call.

// Bench code: `worker1`/`worker2`-style names are intentionally parallel, and
// the loop-counter cast operates on small, bounded benchmark sizes.
#![allow(clippy::similar_names, clippy::cast_possible_truncation)]

use chrono::NaiveDate;
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use worker_matcher::{
    Address, Gender, MatchConfig, MatchingEngine, NicknameTable, SimilarityAlgorithm, Worker,
};

fn dob(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

fn build_alice() -> Worker {
    Worker::builder()
        .uk_nhs_number("943 476 5919")
        .given_name("Alice")
        .middle_name("Marie")
        .family_name("Williams")
        .date_of_birth(dob(1980, 5, 15))
        .gender(Gender::Female)
        .address(
            Address::new()
                .with_line1("10 High Street")
                .with_city("Cardiff")
                .with_postcode("CF10 1AA"),
        )
        .birth_place(Address::new().with_city("Cardiff").with_country("Wales"))
        .phone("07700 900123")
        .email("alice.williams@example.org")
        .build()
}

fn build_alyce_fuzzy(seed: &Worker) -> Worker {
    Worker::builder()
        .uk_nhs_number("943 476 5919")
        .given_name("Alyce")
        .middle_name("Mary")
        .family_name(seed.family_name.clone().unwrap_or_default())
        .date_of_birth(seed.date_of_birth.unwrap())
        .gender(Gender::Female)
        .address(
            Address::new()
                .with_line1("10 High St")
                .with_city("Cardiff")
                .with_postcode("CF10 1AA"),
        )
        .birth_place(Address::new().with_city("Cardiff").with_country("Wales"))
        .phone("+44 7700 900123")
        .email("alice.williams@example.org")
        .build()
}

fn build_unrelated() -> Worker {
    Worker::builder()
        .uk_nhs_number("400 000 0004")
        .given_name("Bob")
        .family_name("Jones")
        .date_of_birth(dob(1965, 11, 30))
        .gender(Gender::Male)
        .address(
            Address::new()
                .with_line1("99 Other Lane")
                .with_city("Glasgow")
                .with_postcode("G1 1AA"),
        )
        .build()
}

fn make_candidate(idx: usize) -> Worker {
    let last = ["Smith", "Jones", "Brown", "Taylor", "Williams", "Davies"];
    let cities = ["London", "Cardiff", "Edinburgh", "Belfast", "Manchester"];
    Worker::builder()
        .uk_nhs_number(format!("943 476 59{:02}", idx % 100))
        .given_name(if idx.is_multiple_of(7) {
            "Alyce"
        } else {
            "Other"
        })
        .family_name(last[idx % last.len()])
        .date_of_birth(dob(1980, 5, (idx % 28 + 1) as u32))
        .gender(if idx.is_multiple_of(2) {
            Gender::Female
        } else {
            Gender::Male
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
    let alice = build_alice();
    let alyce = build_alyce_fuzzy(&alice);
    let unrelated = build_unrelated();
    let engine = MatchingEngine::default_config();

    group.bench_function("identical_clone", |b| {
        let clone = alice.clone();
        b.iter(|| engine.match_workers(black_box(&alice), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_workers(black_box(&alice), black_box(&alyce)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_workers(black_box(&alice), black_box(&unrelated)));
    });
    group.finish();
}

fn bench_deterministic_match(c: &mut Criterion) {
    let alice = build_alice();
    let alyce = build_alyce_fuzzy(&alice);
    let engine = MatchingEngine::default_config();
    c.bench_function("deterministic_match_identifier_hit", |b| {
        b.iter(|| engine.deterministic_match(black_box(&alice), black_box(&alyce)));
    });
}

fn bench_batch_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("rank_one_to_many");
    let engine = MatchingEngine::default_config();
    let query = build_alice();

    for &n in &[10usize, 100, 1000] {
        let candidates: Vec<Worker> = (0..n).map(make_candidate).collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &candidates, |b, cands| {
            b.iter(|| engine.rank_one_to_many(black_box(&query), black_box(cands)));
        });
    }
    group.finish();
}

fn bench_engine_configurations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_variants");
    let alice = build_alice();
    let alyce = build_alyce_fuzzy(&alice);

    let default = MatchingEngine::default_config();
    let strict = MatchingEngine::new(MatchConfig::strict());
    let with_nicknames = MatchingEngine::new(MatchConfig {
        nickname_table: NicknameTable::english(),
        name_algorithm: SimilarityAlgorithm::Combined,
        ..MatchConfig::default()
    });

    group.bench_function("default", |b| {
        b.iter(|| default.match_workers(black_box(&alice), black_box(&alyce)));
    });
    group.bench_function("strict", |b| {
        b.iter(|| strict.match_workers(black_box(&alice), black_box(&alyce)));
    });
    group.bench_function("nickname_table_english", |b| {
        b.iter(|| with_nicknames.match_workers(black_box(&alice), black_box(&alyce)));
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
