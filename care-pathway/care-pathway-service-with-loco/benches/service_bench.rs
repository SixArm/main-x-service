#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `care-pathway-service` pure request-path
//! work: payload validation, record merge, and the Tantivy retrieval
//! that backs duplicate detection.
//!
//! Run with `cargo bench`. Nothing here touches Postgres — these are the
//! CPU-bound halves of a request, which is exactly the part a database
//! benchmark would hide behind I/O.
//!
//! What the groups are for:
//!
//! - **`validation`** — runs on every create and update, and here it
//!   also validates every condition code, so its cost scales with the
//!   coded content of the pathway rather than being a flat check. The
//!   `oversized_arrays` case exercises the SEC-M1 input caps: rejecting
//!   an abusive payload must be cheap.
//! - **`merge`** — a whole-record fold; the scaling case shows the cost
//!   sits in the collections it unions.
//! - **`search`** — `index_one_document` is what every create, update,
//!   and merge pays synchronously; `duplicate_candidates` is what a
//!   duplicate check actually calls.

use std::hint::black_box;

use care_pathway_matcher::{
    CarePathway, CareSetting, CodeSystem, ConditionCode, IdentifierScheme, PathwayIdentifier,
};
use care_pathway_service::{merge, search::SearchEngine, validation};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use uuid::Uuid;

fn cond(system: CodeSystem, code: &str) -> ConditionCode {
    ConditionCode {
        system,
        code: code.into(),
    }
}

/// A fully populated pathway — every validated field present, so
/// validation runs its whole rule set.
fn populated() -> CarePathway {
    let mut p = CarePathway::new("Acute Stroke Care Pathway");
    p.alternate_names = vec!["Stroke pathway".into()];
    p.pathway_code = Some("STR-001".into());
    p.provider_id = Some("ods-RJ1".into());
    p.provider_name = Some("Example NHS Trust".into());
    p.care_setting = Some(CareSetting::Inpatient);
    p.condition_codes = vec![
        cond(CodeSystem::Icd10, "I63"),
        cond(CodeSystem::Snomed, "422504002"),
    ];
    p.interventions = vec!["thrombolysis".into(), "ct angiography".into()];
    p.keywords = vec!["stroke".into(), "neurology".into()];
    p.identifiers = vec![PathwayIdentifier {
        scheme: IdentifierScheme::Doi,
        value: "10.1234/stroke".into(),
    }];
    p.same_as = vec!["https://guidelines.example/stroke".into()];
    p.in_language = vec!["en".into()];
    p
}

/// A payload violating several rules at once — the controller collects
/// every problem in one pass, so this is the realistic bad-request shape.
fn invalid() -> CarePathway {
    let mut p = populated();
    p.name = "   ".into();
    // A malformed ICD-10 code, which is the entity-specific rule here.
    p.condition_codes = vec![
        cond(CodeSystem::Icd10, "not a code"),
        cond(CodeSystem::Snomed, ""),
    ];
    p.keywords = vec![String::new(), "stroke".into()];
    p.identifiers = vec![PathwayIdentifier {
        scheme: IdentifierScheme::Doi,
        value: String::new(),
    }];
    p
}

/// Deliberately over the SEC-M1 caps: the shape an attacker sends to buy
/// O(n·m) scoring work with one cheap request.
fn oversized() -> CarePathway {
    let mut p = populated();
    p.name = "x".repeat(64 * 1024);
    p.interventions = (0..10_000).map(|i| format!("procedure-{i}")).collect();
    p.condition_codes = (0..10_000)
        .map(|i| cond(CodeSystem::Icd10, &format!("I{:02}", i % 100)))
        .collect();
    p
}

/// The `idx`-th distinct pathway, for the index and merge fixtures.
fn variant(idx: usize) -> CarePathway {
    let topics = ["Stroke", "Sepsis", "Diabetes", "Fracture", "Asthma"];
    let codes = ["I63", "A41", "E11", "S72", "J45"];
    let mut p = CarePathway::new(format!("{} Care Pathway {idx}", topics[idx % topics.len()]));
    p.pathway_code = Some(format!("PW-{:04}", idx % 500));
    p.provider_id = Some(format!("ods-R{:02}", idx % 30));
    p.provider_name = Some("Example NHS Trust".into());
    p.care_setting = Some(CareSetting::Inpatient);
    p.condition_codes = vec![cond(CodeSystem::Icd10, codes[idx % codes.len()])];
    p.interventions = vec!["thrombolysis".into()];
    p.keywords = vec!["acute".into(), format!("specialty-{}", idx % 25)];
    p
}

fn bench_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");
    let valid = populated();
    let invalid = invalid();
    let oversized = oversized();

    group.bench_function("valid_payload", |b| {
        b.iter(|| validation::problems(black_box(&valid)));
    });
    group.bench_function("several_problems", |b| {
        b.iter(|| validation::problems(black_box(&invalid)));
    });
    group.bench_function("oversized_arrays", |b| {
        b.iter(|| validation::problems(black_box(&oversized)));
    });
    // The per-code check on its own, since it is the one rule whose cost
    // grows with how thoroughly a pathway is coded.
    let code = cond(CodeSystem::Icd10, "I63.0");
    group.bench_function("one_condition_code", |b| {
        b.iter(|| validation::condition_code_issue(black_box(&code)));
    });
    group.finish();
}

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge");
    let main = populated();
    let duplicate = variant(7);
    group.bench_function("typical_pair", |b| {
        b.iter(|| merge::merge_pathways(black_box(&main), black_box(&duplicate)));
    });

    // Collection union is where merge's cost lives, so scale that.
    let mut big_main = populated();
    big_main.interventions = (0..500).map(|i| format!("procedure-{i}")).collect();
    big_main.keywords = (0..500).map(|i| format!("keyword-{i}")).collect();
    let mut big_dup = populated();
    big_dup.interventions = (250..750).map(|i| format!("procedure-{i}")).collect();
    big_dup.keywords = (250..750).map(|i| format!("keyword-{i}")).collect();
    group.bench_function("many_interventions_half_overlapping", |b| {
        b.iter(|| merge::merge_pathways(black_box(&big_main), black_box(&big_dup)));
    });
    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("temp dir");
    let engine = SearchEngine::new(dir.path()).expect("index opens");
    for i in 0..500 {
        engine
            .index_pathway(Uuid::new_v4(), &variant(i))
            .expect("index write");
    }

    let mut group = c.benchmark_group("search");
    // Its own index, deliberately: measured against the populated one
    // the number would be dominated by accumulated segment state rather
    // than by what a write costs.
    let write_dir = tempfile::tempdir().expect("temp dir");
    let write_engine = SearchEngine::new(write_dir.path()).expect("index opens");
    group.bench_function("index_one_document", |b| {
        let pathway = populated();
        b.iter(|| {
            write_engine
                .index_pathway(black_box(Uuid::new_v4()), black_box(&pathway))
                .expect("index write");
        });
    });
    group.bench_function("exact", |b| {
        b.iter(|| engine.search(black_box("Stroke Care"), 20).expect("search"));
    });
    group.bench_function("fuzzy", |b| {
        b.iter(|| {
            engine
                .fuzzy_search(black_box("Strke Care"), 20)
                .expect("search")
        });
    });
    group.bench_function("phonetic", |b| {
        b.iter(|| {
            engine
                .phonetic_search(black_box("Strohk Kare"), 20)
                .expect("search")
        });
    });

    let probe = populated();
    for &limit in &[10usize, 50] {
        group.throughput(Throughput::Elements(limit as u64));
        group.bench_with_input(
            BenchmarkId::new("duplicate_candidates", limit),
            &limit,
            |b, &limit| {
                b.iter(|| {
                    engine
                        .candidates(black_box(&probe), limit)
                        .expect("candidates")
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_validation, bench_merge, bench_search);
criterion_main!(benches);
