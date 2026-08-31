#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `course-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator exercises: single-pair probabilistic matching, the
//! deterministic short-circuit, batch ranking, and the cost each config
//! preset adds. Numbers are absolute time per call.
//!
//! The ranking group sets `Throughput::Elements`, so Criterion reports
//! per-candidate cost — which is what makes super-linear scaling visible
//! rather than something you infer from three separate absolute numbers.

use course_matcher::{
    Course, CourseIdentifier, EducationalLevel, IdentifierScheme, LearningResourceType,
    MatchConfig, MatchingEngine,
};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

/// Terse `CourseIdentifier` constructor.
fn ident(scheme: IdentifierScheme, value: &str) -> CourseIdentifier {
    CourseIdentifier {
        scheme,
        value: value.into(),
    }
}

/// The reference course: every scored component populated (name,
/// provider-scoped code, level, type, keywords, teaches) so a benchmark
/// exercises the whole pipeline rather than the cheap early-outs.
fn build_reference() -> Course {
    let mut c = Course::new("Introduction to Computer Science");
    c.alternate_names = vec!["Intro to CS".into()];
    c.course_code = Some("CS101".into());
    c.provider_id = Some("ror-021nxhr62".into());
    c.provider_name = Some("Example University".into());
    c.educational_level = Some(EducationalLevel::Undergraduate);
    c.learning_resource_type = Some(LearningResourceType::Lecture);
    c.keywords = vec![
        "computer science".into(),
        "programming".into(),
        "algorithms".into(),
    ];
    c.teaches = vec!["recursion".into(), "data structures".into()];
    c.in_language = vec!["en".into()];
    c
}

/// A near-duplicate: the "same course, different catalogue" case that
/// drives the full probabilistic path — fuzzy name, differently-spaced
/// code, partially overlapping keywords, and **no** shared provider id,
/// so the deterministic rule cannot fire.
fn build_near_duplicate() -> Course {
    let mut c = Course::new("Intro to Computer Sciences");
    c.course_code = Some("cs 101".into());
    c.provider_name = Some("Example Univ.".into());
    c.educational_level = Some(EducationalLevel::Undergraduate);
    c.keywords = vec!["computer science".into(), "algorithms".into()];
    c.teaches = vec!["data structures".into()];
    c
}

/// A clearly different course, for the common "no match" path where most
/// sub-scores are near zero.
fn build_unrelated() -> Course {
    let mut c = Course::new("Introductory Basket Weaving");
    c.course_code = Some("ART900".into());
    c.provider_id = Some("ror-000000000".into());
    c.educational_level = Some(EducationalLevel::Vocational);
    c.keywords = vec!["crafts".into(), "textiles".into()];
    c
}

/// Deterministically synthesise the `idx`-th candidate, so a candidate
/// list of any size is reproducible without randomness.
fn make_candidate(idx: usize) -> Course {
    let subjects = ["Computer Science", "Physics", "History", "Biology", "Law"];
    let mut c = Course::new(format!("{} {}", subjects[idx % subjects.len()], idx % 400));
    c.course_code = Some(format!("SUB{:03}", idx % 500));
    c.provider_id = Some(format!("ror-{:09}", idx % 25));
    c.educational_level = Some(if idx.is_multiple_of(3) {
        EducationalLevel::Undergraduate
    } else {
        EducationalLevel::Graduate
    });
    c.keywords = vec!["computer science".into(), format!("topic-{}", idx % 40)];
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
        b.iter(|| engine.match_courses(black_box(&reference), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_courses(black_box(&reference), black_box(&near)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_courses(black_box(&reference), black_box(&unrelated)));
    });
    group.finish();
}

/// The deterministic paths, which are the cheapest decisions the engine
/// makes and the ones a duplicate check hits most often: a shared DOI
/// (R-0) and a shared provider-scoped course code (R-1).
fn bench_deterministic(c: &mut Criterion) {
    let mut group = c.benchmark_group("deterministic");
    let engine = MatchingEngine::default_config();

    let mut a = build_reference();
    let mut b = build_unrelated();
    a.identifiers
        .push(ident(IdentifierScheme::Doi, "10.1234/x"));
    b.identifiers
        .push(ident(IdentifierScheme::Doi, "10.1234/x"));
    group.bench_function("shared_doi", |bencher| {
        bencher.iter(|| engine.match_courses(black_box(&a), black_box(&b)));
    });

    let left = build_reference();
    let mut right = build_near_duplicate();
    right.provider_id.clone_from(&left.provider_id);
    group.bench_function("same_provider_course_code", |bencher| {
        bencher.iter(|| engine.match_courses(black_box(&left), black_box(&right)));
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
        let candidates: Vec<Course> = (0..n).map(make_candidate).collect();
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
            b.iter(|| engine.match_courses(black_box(&reference), black_box(&near)));
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
