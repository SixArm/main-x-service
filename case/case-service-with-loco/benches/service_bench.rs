#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `case-service` pure request-path work:
//! payload validation, record merge, and the Tantivy retrieval that
//! backs duplicate detection.
//!
//! Run with `cargo bench`. Nothing here touches Postgres — these are the
//! CPU-bound halves of a request, which is exactly the part a database
//! benchmark would hide behind I/O.
//!
//! What the groups are for:
//!
//! - **`validation`** — runs on every create and update. The
//!   `oversized_arrays` case exercises the SEC-M1 input caps: rejecting
//!   an abusive payload must be *cheap*, because the whole point of the
//!   caps is to stop a caller buying expensive work with a small
//!   request.
//! - **`merge`** — a whole-record fold, allocation-heavy by nature; the
//!   `many_subjects` case shows how it scales with the collections it
//!   unions rather than with the record count.
//! - **`search`** — the duplicate-check path is `candidates()`, which
//!   blocks on the index instead of scanning the table. Benchmarked
//!   against a populated index so the number means something.

use std::hint::black_box;

use case_matcher::{Case, CaseIdentifier, CaseStatus, CaseType, IdentifierScheme, Priority};
use case_service::{merge, search::SearchEngine, validation};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use uuid::Uuid;

/// A fully populated case — every validated field present, so validation
/// runs its whole rule set rather than stopping at a missing title.
fn populated() -> Case {
    let mut c = Case::new("Housing benefit appeal — J. Smith");
    c.alternate_titles = vec!["HB appeal Smith".into(), "Smith appeal".into()];
    c.case_number = Some("HB-2024-001234".into());
    c.agency_id = Some("agency-1".into());
    c.agency_name = Some("Example Borough Council".into());
    c.case_type = Some(CaseType::Housing);
    c.status = Some(CaseStatus::InProgress);
    c.priority = Some(Priority::Normal);
    c.opened_date = Some("2024-03-11".into());
    c.subjects = vec!["person:0c4f1e2a".into(), "person:7b3d9911".into()];
    c.keywords = vec!["housing".into(), "benefit".into(), "appeal".into()];
    c.identifiers = vec![CaseIdentifier {
        scheme: IdentifierScheme::Docket,
        value: "2024-CV-001234".into(),
    }];
    c.same_as = vec!["https://courts.example/case/2024-CV-001234".into()];
    c.in_language = vec!["en".into()];
    c
}

/// A payload that violates several rules at once — the controller
/// collects every problem in one pass, so this is the realistic "bad
/// request" shape rather than a single-fault one.
fn invalid() -> Case {
    let mut c = populated();
    c.title = "   ".into();
    c.opened_date = Some("2024-13-45".into());
    c.subjects = vec![String::new(), "person:0c4f1e2a".into()];
    c.identifiers = vec![CaseIdentifier {
        scheme: IdentifierScheme::Docket,
        value: String::new(),
    }];
    c
}

/// A payload deliberately over the SEC-M1 caps: a huge title and a huge
/// keyword array, the shape an attacker sends to buy O(n·m) scoring work
/// with one cheap request.
fn oversized() -> Case {
    let mut c = populated();
    c.title = "x".repeat(64 * 1024);
    c.keywords = (0..10_000).map(|i| format!("keyword-{i}")).collect();
    c.subjects = (0..10_000).map(|i| format!("person:{i:08x}")).collect();
    c
}

/// The `idx`-th distinct case, for populating the search index and the
/// merge-scaling fixtures.
fn variant(idx: usize) -> Case {
    let topics = ["Housing benefit", "Council tax", "Planning", "Licensing"];
    let mut c = Case::new(format!("{} matter {}", topics[idx % topics.len()], idx));
    c.case_number = Some(format!("HB-2024-{idx:06}"));
    c.agency_id = Some(format!("agency-{}", idx % 20));
    c.agency_name = Some("Example Borough Council".into());
    c.case_type = Some(CaseType::Housing);
    c.subjects = vec![format!("person:{idx:08x}")];
    c.keywords = vec!["housing".into(), format!("tag-{}", idx % 30)];
    c
}

/// Payload validation — every create and update pays this.
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
    group.finish();
}

/// Record merge — the fold that produces the survivor, plus its
/// transferred-data snapshot.
fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge");
    let main = populated();
    let duplicate = variant(7);

    group.bench_function("typical_pair", |b| {
        b.iter(|| merge::merge_cases(black_box(&main), black_box(&duplicate)));
    });

    // Collection union is where merge's cost actually lives, so scale the
    // collections rather than anything else.
    let mut big_main = populated();
    big_main.subjects = (0..500).map(|i| format!("person:{i:08x}")).collect();
    big_main.keywords = (0..500).map(|i| format!("keyword-{i}")).collect();
    let mut big_dup = populated();
    big_dup.subjects = (250..750).map(|i| format!("person:{i:08x}")).collect();
    big_dup.keywords = (250..750).map(|i| format!("keyword-{i}")).collect();
    group.bench_function("many_subjects_half_overlapping", |b| {
        b.iter(|| merge::merge_cases(black_box(&big_main), black_box(&big_dup)));
    });
    group.finish();
}

/// Retrieval against a populated index: the exact / fuzzy / phonetic
/// queries the API exposes, and `candidates`, which is what a duplicate
/// check actually calls.
fn bench_search(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("temp dir");
    let engine = SearchEngine::new(dir.path()).expect("index opens");
    for i in 0..500 {
        engine
            .index_case(Uuid::new_v4(), &variant(i))
            .expect("index write");
    }

    let mut group = c.benchmark_group("search");
    // Indexing one record is what every create / update / merge pays
    // synchronously, so it belongs in the same group as the queries it
    // feeds — a retrieval benchmark that ignores the write side prices
    // half the path.
    // Its own index, deliberately: measured against the 500-segment one
    // below, the number would be dominated by accumulated segment state
    // rather than by what a write costs.
    let write_dir = tempfile::tempdir().expect("temp dir");
    let write_engine = SearchEngine::new(write_dir.path()).expect("index opens");
    group.bench_function("index_one_document", |b| {
        let case = populated();
        b.iter(|| {
            write_engine
                .index_case(black_box(Uuid::new_v4()), black_box(&case))
                .expect("index write");
        });
    });
    group.bench_function("exact", |b| {
        b.iter(|| {
            engine
                .search(black_box("Housing benefit"), 20)
                .expect("search")
        });
    });
    group.bench_function("fuzzy", |b| {
        b.iter(|| {
            engine
                .fuzzy_search(black_box("Housing benefitt"), 20)
                .expect("search")
        });
    });
    group.bench_function("phonetic", |b| {
        b.iter(|| {
            engine
                .phonetic_search(black_box("Howsing benifit"), 20)
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
