#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `care-pathway-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator exercises: single-pair probabilistic matching, the
//! deterministic short-circuit, batch ranking, and the cost each config
//! preset adds. Numbers are absolute time per call.
//!
//! The ranking group sets `Throughput::Elements`, so Criterion reports
//! per-candidate cost — which is what makes super-linear scaling visible
//! rather than something you infer from three separate absolute numbers.

use care_pathway_matcher::{
    CarePathway, CareSetting, CodeSystem, ConditionCode, IdentifierScheme, MatchConfig,
    MatchingEngine, PathwayIdentifier,
};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

/// Terse `PathwayIdentifier` constructor.
fn ident(scheme: IdentifierScheme, value: &str) -> PathwayIdentifier {
    PathwayIdentifier {
        scheme,
        value: value.into(),
    }
}

/// Terse `ConditionCode` constructor.
fn cond(system: CodeSystem, code: &str) -> ConditionCode {
    ConditionCode {
        system,
        code: code.into(),
    }
}

/// The reference pathway: every scored component populated (name,
/// provider-scoped code, setting, condition codes, interventions,
/// keywords) so a benchmark exercises the whole pipeline.
fn build_reference() -> CarePathway {
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
    p.interventions = vec![
        "thrombolysis".into(),
        "ct angiography".into(),
        "swallow screen".into(),
    ];
    p.keywords = vec!["stroke".into(), "neurology".into(), "acute".into()];
    p.in_language = vec!["en".into()];
    p
}

/// A near-duplicate: the "same pathway, different trust" case that
/// drives the full probabilistic path — fuzzy name, differently-spaced
/// code, partially overlapping codes and interventions, and **no**
/// shared provider id, so the deterministic rule cannot fire.
fn build_near_duplicate() -> CarePathway {
    let mut p = CarePathway::new("Acute Stroke Pathway");
    p.pathway_code = Some("str 001".into());
    p.provider_name = Some("Example Trust".into());
    p.care_setting = Some(CareSetting::Inpatient);
    p.condition_codes = vec![cond(CodeSystem::Icd10, "I63")];
    p.interventions = vec!["thrombolysis".into(), "swallow screening".into()];
    p.keywords = vec!["stroke".into(), "acute".into()];
    p
}

/// A clearly different pathway, for the common "no match" path.
fn build_unrelated() -> CarePathway {
    let mut p = CarePathway::new("Elective Hip Replacement Pathway");
    p.pathway_code = Some("ORT-042".into());
    p.provider_id = Some("ods-RXX".into());
    p.care_setting = Some(CareSetting::Outpatient);
    p.condition_codes = vec![cond(CodeSystem::Icd10, "M16")];
    p.interventions = vec!["arthroplasty".into()];
    p.keywords = vec!["orthopaedics".into()];
    p
}

/// Deterministically synthesise the `idx`-th candidate, so a candidate
/// list of any size is reproducible without randomness.
fn make_candidate(idx: usize) -> CarePathway {
    let topics = ["Stroke", "Sepsis", "Diabetes", "Fracture", "Asthma"];
    let codes = ["I63", "A41", "E11", "S72", "J45"];
    let mut p = CarePathway::new(format!(
        "{} Care Pathway {}",
        topics[idx % topics.len()],
        idx % 300
    ));
    p.pathway_code = Some(format!("PW-{:04}", idx % 500));
    p.provider_id = Some(format!("ods-R{:02}", idx % 30));
    p.care_setting = Some(if idx.is_multiple_of(2) {
        CareSetting::Inpatient
    } else {
        CareSetting::Community
    });
    p.condition_codes = vec![cond(CodeSystem::Icd10, codes[idx % codes.len()])];
    p.interventions = vec!["thrombolysis".into(), format!("procedure-{}", idx % 40)];
    p.keywords = vec!["acute".into(), format!("specialty-{}", idx % 25)];
    p
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
        b.iter(|| engine.match_care_pathways(black_box(&reference), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_care_pathways(black_box(&reference), black_box(&near)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_care_pathways(black_box(&reference), black_box(&unrelated)));
    });
    group.finish();
}

/// The deterministic paths — the cheapest decisions the engine makes and
/// the ones a duplicate check hits most often: a shared DOI (R-0) and a
/// shared provider-scoped pathway code (R-1).
fn bench_deterministic(c: &mut Criterion) {
    let mut group = c.benchmark_group("deterministic");
    let engine = MatchingEngine::default_config();

    let mut a = build_reference();
    let mut b = build_unrelated();
    a.identifiers
        .push(ident(IdentifierScheme::Doi, "10.1234/stroke"));
    b.identifiers
        .push(ident(IdentifierScheme::Doi, "10.1234/stroke"));
    group.bench_function("shared_doi", |bencher| {
        bencher.iter(|| engine.match_care_pathways(black_box(&a), black_box(&b)));
    });

    let left = build_reference();
    let mut right = build_near_duplicate();
    right.provider_id.clone_from(&left.provider_id);
    group.bench_function("same_provider_pathway_code", |bencher| {
        bencher.iter(|| engine.match_care_pathways(black_box(&left), black_box(&right)));
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
        let candidates: Vec<CarePathway> = (0..n).map(make_candidate).collect();
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
            b.iter(|| engine.match_care_pathways(black_box(&reference), black_box(&near)));
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
