#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `case-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator exercises: single-pair probabilistic matching, the
//! deterministic short-circuit, batch ranking, and the cost each config
//! preset adds. Numbers are absolute time per call.
//!
//! The ranking group sets `Throughput::Elements`, so Criterion reports
//! per-candidate cost — which is what makes super-linear scaling visible
//! rather than something you infer from three separate absolute numbers.

use case_matcher::{
    Case, CaseIdentifier, CaseStatus, CaseType, IdentifierScheme, MatchConfig, MatchingEngine,
    Priority,
};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

/// Terse `CaseIdentifier` constructor.
fn ident(scheme: IdentifierScheme, value: &str) -> CaseIdentifier {
    CaseIdentifier {
        scheme,
        value: value.into(),
    }
}

/// The reference case: every scored component populated (title,
/// alternates, agency-scoped number, type/status, subjects, keywords)
/// so a benchmark exercises the whole pipeline. `subjects` is the
/// heaviest component — an overlap set, so its cost grows with the
/// number of involved parties.
fn build_reference() -> Case {
    let mut c = Case::new("Housing benefit appeal — J. Smith");
    c.alternate_titles = vec!["HB appeal Smith".into()];
    c.case_number = Some("HB-2024-001234".into());
    c.agency_id = Some("agency-1".into());
    c.agency_name = Some("Example Borough Council".into());
    c.case_type = Some(CaseType::Housing);
    c.status = Some(CaseStatus::InProgress);
    c.priority = Some(Priority::Normal);
    c.opened_date = Some("2024-03-11".into());
    c.subjects = vec![
        "person:0c4f1e2a".into(),
        "person:7b3d9911".into(),
        "person:1a2b3c4d".into(),
    ];
    c.keywords = vec!["housing".into(), "benefit".into(), "appeal".into()];
    c.in_language = vec!["en".into()];
    c
}

/// A near-duplicate: the "same matter, different intake system" case
/// that drives the full probabilistic path — fuzzy title, re-formatted
/// case number, partially overlapping subjects, and a **different**
/// agency, so the agency-scoped rule cannot fire.
fn build_near_duplicate() -> Case {
    let mut c = Case::new("Housing Benefit Appeal (Smith, J)");
    c.case_number = Some("hb 2024 001234".into());
    c.agency_id = Some("agency-2".into());
    c.agency_name = Some("Example Borough".into());
    c.case_type = Some(CaseType::Housing);
    c.status = Some(CaseStatus::Open);
    c.subjects = vec!["person:0c4f1e2a".into(), "person:9999aaaa".into()];
    c.keywords = vec!["housing".into(), "appeal".into()];
    c
}

/// A clearly different case, for the common "no match" path.
fn build_unrelated() -> Case {
    let mut c = Case::new("Licensing application — Riverside Cafe");
    c.case_number = Some("LIC-2019-0042".into());
    c.agency_id = Some("agency-9".into());
    c.case_type = Some(CaseType::Licensing);
    c.status = Some(CaseStatus::Closed);
    c.subjects = vec!["person:deadbeef".into()];
    c.keywords = vec!["licensing".into(), "premises".into()];
    c
}

/// Deterministically synthesise the `idx`-th candidate, so a candidate
/// list of any size is reproducible without randomness.
fn make_candidate(idx: usize) -> Case {
    let topics = [
        "Housing benefit",
        "Council tax",
        "Planning",
        "Licensing",
        "Appeal",
    ];
    let types = [
        CaseType::Housing,
        CaseType::Benefit,
        CaseType::Legal,
        CaseType::Licensing,
        CaseType::Appeal,
    ];
    let mut c = Case::new(format!(
        "{} matter {}",
        topics[idx % topics.len()],
        idx % 400
    ));
    c.case_number = Some(format!("HB-2024-{:06}", idx % 1000));
    c.agency_id = Some(format!("agency-{}", idx % 20));
    c.case_type = Some(types[idx % types.len()].clone());
    c.status = Some(if idx.is_multiple_of(3) {
        CaseStatus::Open
    } else {
        CaseStatus::Closed
    });
    c.subjects = vec![
        "person:0c4f1e2a".into(),
        format!("person:{:08x}", idx % 5000),
    ];
    c.keywords = vec!["housing".into(), format!("tag-{}", idx % 30)];
    c
}

/// Single-pair probabilistic matching across three regimes: an identical
/// clone (best case), a fuzzy near-match (full pipeline), and an
/// unrelated pair (mostly-zero sub-scores).
fn bench_match_pair(c: &mut Criterion) {
    let mut group = c.benchmark_group("match_pair");
    let reference = build_reference();
    let near = build_near_duplicate();
    let unrelated = build_unrelated();
    let engine = MatchingEngine::default_config();

    group.bench_function("identical_clone", |b| {
        let clone = reference.clone();
        b.iter(|| engine.match_cases(black_box(&reference), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_cases(black_box(&reference), black_box(&near)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_cases(black_box(&reference), black_box(&unrelated)));
    });
    group.finish();
}

/// The deterministic paths — the cheapest decisions the engine makes and
/// the ones a duplicate check hits most often: a shared docket (globally
/// unique) and a shared agency-scoped case number.
fn bench_deterministic(c: &mut Criterion) {
    let mut group = c.benchmark_group("deterministic");
    let engine = MatchingEngine::default_config();

    let mut a = build_reference();
    let mut b = build_unrelated();
    a.identifiers
        .push(ident(IdentifierScheme::Docket, "2024-CV-001234"));
    b.identifiers
        .push(ident(IdentifierScheme::Docket, "2024-CV-001234"));
    group.bench_function("shared_docket", |bencher| {
        bencher.iter(|| engine.match_cases(black_box(&a), black_box(&b)));
    });

    let left = build_reference();
    let mut right = build_near_duplicate();
    right.agency_id.clone_from(&left.agency_id);
    group.bench_function("same_agency_case_number", |bencher| {
        bencher.iter(|| engine.match_cases(black_box(&left), black_box(&right)));
    });
    group.finish();
}

/// `rank` at 10/100/1000 candidates, with `Throughput::Elements` so
/// Criterion reports per-candidate cost.
fn bench_rank(c: &mut Criterion) {
    let mut group = c.benchmark_group("rank");
    let engine = MatchingEngine::default_config();
    let query = build_reference();

    for &n in &[10usize, 100, 1000] {
        let candidates: Vec<Case> = (0..n).map(make_candidate).collect();
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(BenchmarkId::from_parameter(n), &candidates, |b, cands| {
            b.iter(|| engine.rank(black_box(&query), black_box(cands)));
        });
    }
    group.finish();
}

/// The same fuzzy pair under each shipped preset, exposing what the
/// chosen thresholds and weights cost per call.
fn bench_config_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_variants");
    let reference = build_reference();
    let near = build_near_duplicate();

    for (label, engine) in [
        ("default", MatchingEngine::default_config()),
        ("strict", MatchingEngine::new(MatchConfig::strict())),
        ("lenient", MatchingEngine::new(MatchConfig::lenient())),
    ] {
        group.bench_function(label, |b| {
            b.iter(|| engine.match_cases(black_box(&reference), black_box(&near)));
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_match_pair,
    bench_deterministic,
    bench_rank,
    bench_config_variants
);
criterion_main!(benches);
