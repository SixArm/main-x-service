//! Bridge-path benchmarks: service-side `Worker` → `to_matcher_worker` →
//! `worker_matcher::MatchingEngine::match_workers`.
//!
//! Measures the cost of the adapter projection and the end-to-end bridge so
//! any future change to the adapter or the matcher's hot path shows up as a
//! clear regression. Three groups:
//!
//! - `bridge_adapter_only` — `to_matcher_worker` on minimal and rich records.
//! - `bridge_end_to_end`   — adapter + engine call (the realistic dedup path).
//! - `bridge_one_to_many`  — single query vs. 100 candidates.

use chrono::{NaiveDate, Utc};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use uuid::Uuid;

use worker_service::matching::adapter::to_matcher_worker;
use worker_service::matching::matcher_lib::{MatchConfig, MatchingEngine};
use worker_service::models::{
    Address, AddressUse, ContactPoint, ContactPointSystem, ContactPointUse,
    DocumentType, Gender, HumanName, Identifier, IdentifierType, IdentityDocument,
    Worker,
};

// -------- fixtures -----------------------------------------------------------

/// A bare [`Worker`] (name + gender only) — the cheap end of the adapter cost.
fn minimal_worker() -> Worker {
    let mut w = Worker::new(
        HumanName {
            use_type: None,
            family: "Patel".into(),
            given: vec!["Asha".into()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    );
    w.id = Uuid::new_v4();
    w.created_at = Utc::now();
    w.updated_at = w.created_at;
    w
}

/// A fully-populated [`Worker`] (identifiers, telecom, addresses, document,
/// tax id) exercising every adapter routing branch — the expensive case.
fn rich_worker() -> Worker {
    let mut w = minimal_worker();
    w.birth_date = NaiveDate::from_ymd_opt(1970, 4, 1);
    w.tax_id = Some("123-45-6789".into());
    w.identifiers.push(Identifier::new(
        IdentifierType::Other,
        "https://fhir.nhs.uk/Id/nhs-number".into(),
        "943 476 5919".into(),
    ));
    w.identifiers.push(Identifier::new(
        IdentifierType::NPI,
        "http://hl7.org/fhir/sid/us-npi".into(),
        "1234567893".into(),
    ));
    w.telecom.push(ContactPoint {
        system: ContactPointSystem::Phone,
        value: "+1-415-555-0100".into(),
        use_type: Some(ContactPointUse::Work),
    });
    w.telecom.push(ContactPoint {
        system: ContactPointSystem::Email,
        value: "asha@example.com".into(),
        use_type: Some(ContactPointUse::Work),
    });
    for city in ["Boston", "Cambridge", "Somerville"] {
        w.addresses.push(Address {
            use_type: Some(AddressUse::Work),
            line1: Some("1 Test St".into()),
            line2: None,
            city: Some(city.into()),
            state: Some("MA".into()),
            postal_code: Some("02108".into()),
            country: Some("US".into()),
        });
    }
    w.documents.push(IdentityDocument {
        document_type: DocumentType::Passport,
        number: "X12345678".into(),
        issuing_country: Some("US".into()),
        issuing_authority: None,
        issue_date: NaiveDate::from_ymd_opt(2020, 1, 1),
        expiry_date: NaiveDate::from_ymd_opt(2030, 1, 1),
        verified: true,
    });
    w
}

/// The default-config [`MatchingEngine`] used by the bridge benchmarks.
fn engine() -> MatchingEngine {
    MatchingEngine::new(MatchConfig::default())
}

// =============================================================================
// Group 1: adapter projection cost (minimal vs rich record)
// =============================================================================

/// Benchmarks the adapter projection alone (`to_matcher_worker`) on minimal
/// and rich records.
fn bench_adapter_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_adapter_only");
    let minimal = minimal_worker();
    let rich = rich_worker();

    group.bench_function("minimal", |b| {
        b.iter(|| {
            let m = to_matcher_worker(black_box(&minimal));
            black_box(m);
        });
    });
    group.bench_function("rich", |b| {
        b.iter(|| {
            let m = to_matcher_worker(black_box(&rich));
            black_box(m);
        });
    });

    group.finish();
}

// =============================================================================
// Group 2: end-to-end bridge — adapter + engine
// =============================================================================

/// Benchmarks the full bridge (adapter + engine) on cloned minimal and rich
/// pairs.
fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_end_to_end");
    let engine = engine();
    let minimal_a = minimal_worker();
    let minimal_b = minimal_a.clone();
    let rich_a = rich_worker();
    let rich_b = rich_a.clone();

    group.bench_function("minimal_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_workers(
                &to_matcher_worker(black_box(&minimal_a)),
                &to_matcher_worker(black_box(&minimal_b)),
            );
            black_box(r);
        });
    });
    group.bench_function("rich_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_workers(
                &to_matcher_worker(black_box(&rich_a)),
                &to_matcher_worker(black_box(&rich_b)),
            );
            black_box(r);
        });
    });

    group.finish();
}

// =============================================================================
// Group 3: one-vs-many — the realistic dedup-on-create path
// =============================================================================

/// Benchmarks the realistic dedup path: one query scored against 10/50/100
/// varied candidates, taking the best score.
fn bench_one_to_many(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_one_to_many");
    let engine = engine();
    let query = rich_worker();

    // 100 candidates — varied so the matcher can't trivially short-circuit.
    let candidates: Vec<Worker> = (0..100)
        .map(|i| {
            let mut p = rich_worker();
            p.id = Uuid::new_v4();
            p.name.given[0] = format!("Variant{}", i);
            p
        })
        .collect();

    for &n in &[10usize, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let qm = to_matcher_worker(black_box(&query));
                let mut best = 0.0_f64;
                for c in candidates.iter().take(n) {
                    let cm = to_matcher_worker(c);
                    let r = engine.match_workers(&qm, &cm);
                    if r.score > best {
                        best = r.score;
                    }
                }
                black_box(best);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_adapter_only,
    bench_end_to_end,
    bench_one_to_many,
);
criterion_main!(benches);
