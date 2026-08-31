#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `person-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator will exercise: single-pair probabilistic matching,
//! deterministic matching, and the batch ranking entry point. Numbers
//! are reported in absolute time per call.

use chrono::NaiveDate;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use person_matcher::{
    Address, Gender, MatchConfig, MatchingEngine, NicknameTable, Person, SimilarityAlgorithm,
};
use std::hint::black_box;

/// Shorthand for a chrono `NaiveDate`, keeping the fixture builders terse.
fn dob(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// The "reference" person: a fully populated record (identifier, names,
/// DOB, gender, two addresses, phone, email) so benchmarks exercise every
/// scoring sub-path rather than just the cheap early-outs.
fn build_alice() -> Person {
    Person::builder()
        .united_kingdom_national_health_service_number("943 476 5919")
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

/// A near-duplicate of `seed`: same identifier and DOB, but with a fuzzy
/// given name (`Alyce`), an abbreviated address line, and a re-formatted
/// phone number — i.e. the realistic "same person, different system" case
/// that drives the full probabilistic pipeline plus normalisation.
fn build_alyce_fuzzy(seed: &Person) -> Person {
    Person::builder()
        .united_kingdom_national_health_service_number("943 476 5919")
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

/// A clearly different person (different identifier, name, DOB, location)
/// to benchmark the common "no match" path where most sub-scores are low.
fn build_unrelated() -> Person {
    Person::builder()
        .united_kingdom_national_health_service_number("400 000 0004")
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

/// Deterministically synthesise the `idx`-th candidate for the batch-ranking
/// benchmark. Fields are derived from `idx` (round-robin surnames/cities,
/// every 7th record carries the matching `Alyce` given name) so a candidate
/// list of any size is reproducible without randomness.
fn make_candidate(idx: usize) -> Person {
    let last = ["Smith", "Jones", "Brown", "Taylor", "Williams", "Davies"];
    let cities = ["London", "Cardiff", "Edinburgh", "Belfast", "Manchester"];
    Person::builder()
        .united_kingdom_national_health_service_number(format!("943 476 59{:02}", idx % 100))
        .given_name(if idx.is_multiple_of(7) {
            "Alyce"
        } else {
            "Other"
        })
        .family_name(last[idx % last.len()])
        .date_of_birth(dob(
            1980,
            5,
            u32::try_from(idx % 28 + 1).unwrap_or(u32::MAX),
        ))
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

/// Benchmark single-pair probabilistic matching across three regimes:
/// an identical clone (best case), a fuzzy near-match (full pipeline),
/// and an unrelated pair (mostly-zero sub-scores).
fn bench_match_pair(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_pair");
    let alice = build_alice();
    let duplicate = build_alyce_fuzzy(&alice);
    let unrelated = build_unrelated();
    let engine = MatchingEngine::default_config();

    group.bench_function("identical_clone", |b| {
        let clone = alice.clone();
        b.iter(|| engine.match_persons(black_box(&alice), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_persons(black_box(&alice), black_box(&duplicate)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_persons(black_box(&alice), black_box(&unrelated)));
    });
    group.finish();
}

/// Benchmark the deterministic path where a shared national identifier
/// short-circuits to a hit — the cheapest match decision the engine makes.
fn bench_deterministic_match(c: &mut Criterion) {
    let alice = build_alice();
    let duplicate = build_alyce_fuzzy(&alice);
    let engine = MatchingEngine::default_config();
    c.bench_function("deterministic_match_identifier_hit", |b| {
        b.iter(|| engine.deterministic_match(black_box(&alice), black_box(&duplicate)));
    });
}

/// Benchmark `rank_one_to_many` at 10/100/1000 candidates with
/// `Throughput::Elements` set, so Criterion reports per-candidate cost and
/// any super-linear scaling shows up directly.
fn bench_batch_ranking(c: &mut Criterion) {
    let mut group = c.benchmark_group("rank_one_to_many");
    let engine = MatchingEngine::default_config();
    let query = build_alice();

    for &n in &[10usize, 100, 1000] {
        let candidates: Vec<Person> = (0..n).map(make_candidate).collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &candidates, |b, cands| {
            b.iter(|| engine.rank_one_to_many(black_box(&query), black_box(cands)));
        });
    }
    group.finish();
}

/// Benchmark the same fuzzy pair under three configs (default, strict,
/// and an English nickname table) to expose the per-call cost the chosen
/// preset adds — notably the extra nickname-table lookup.
fn bench_engine_configurations(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_variants");
    let alice = build_alice();
    let duplicate = build_alyce_fuzzy(&alice);

    let default = MatchingEngine::default_config();
    let strict = MatchingEngine::new(MatchConfig::strict());
    let with_nicknames = MatchingEngine::new(MatchConfig {
        nickname_table: NicknameTable::english(),
        name_algorithm: SimilarityAlgorithm::Combined,
        ..MatchConfig::default()
    });

    group.bench_function("default", |b| {
        b.iter(|| default.match_persons(black_box(&alice), black_box(&duplicate)));
    });
    group.bench_function("strict", |b| {
        b.iter(|| strict.match_persons(black_box(&alice), black_box(&duplicate)));
    });
    group.bench_function("nickname_table_english", |b| {
        b.iter(|| with_nicknames.match_persons(black_box(&alice), black_box(&duplicate)));
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
