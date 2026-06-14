#![warn(clippy::pedantic)]

//! Bridge-path benchmarks: service-side `Person` → `to_matcher_person` →
//! `person_matcher::MatchingEngine::match_persons`.
//!
//! Measures the cost of the adapter projection and the end-to-end
//! bridge so any future change to the adapter or the matcher's hot path
//! shows up as a clear regression. Three groups:
//!
//! - `bridge_adapter_only` — `to_matcher_person` on minimal and rich records.
//! - `bridge_end_to_end`   — adapter + engine call (the realistic dedup path).
//! - `bridge_one_to_many`  — single query vs. 100 candidates.

use chrono::{NaiveDate, Utc};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use uuid::Uuid;

use person_service::matching::adapter::to_matcher_person;
use person_service::matching::matcher_lib::{MatchConfig, MatchingEngine};
use person_service::models::{
    Address, AddressUse, ContactPoint, ContactPointSystem, ContactPointUse, DocumentType, Gender,
    HumanName, Identifier, IdentifierType, IdentityDocument, Person,
};

// -------- fixtures -----------------------------------------------------------

/// A bare-bones person (name + gender + ids/timestamps) for the cheap
/// projection path.
fn minimal_person() -> Person {
    let mut p = Person::new(
        HumanName {
            use_type: None,
            family: "Smith".into(),
            given: vec!["John".into()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Male,
    );
    p.id = Uuid::new_v4();
    p.created_at = Utc::now();
    p.updated_at = p.created_at;
    p
}

/// A fully-populated person (DOB, tax id, 3 identifiers, 3 telecoms, 3
/// addresses, a passport) that exercises every adapter routing branch.
fn rich_person() -> Person {
    let mut p = minimal_person();
    p.birth_date = Some(NaiveDate::from_ymd_opt(1980, 5, 15).unwrap());
    p.tax_id = Some("123-45-6789".into());

    // 3 identifiers — exercises the routing loop.
    p.identifiers.push(Identifier::new(
        IdentifierType::Other,
        "https://fhir.nhs.uk/Id/nhs-number".into(),
        "943 476 5919".into(),
    ));
    p.identifiers.push(Identifier::new(
        IdentifierType::SSN,
        "http://hl7.org/fhir/sid/us-ssn".into(),
        "123-45-6789".into(),
    ));
    p.identifiers.push(Identifier::new(
        IdentifierType::Other,
        "https://uidai.gov.in/aadhaar".into(),
        "234123412346".into(),
    ));

    // 3 telecom entries — exercises the first-of-each-system selection.
    p.telecom.push(ContactPoint {
        system: ContactPointSystem::Phone,
        value: "+44 20 7946 0958".into(),
        use_type: Some(ContactPointUse::Home),
    });
    p.telecom.push(ContactPoint {
        system: ContactPointSystem::Email,
        value: "john@example.com".into(),
        use_type: Some(ContactPointUse::Home),
    });
    p.telecom.push(ContactPoint {
        system: ContactPointSystem::Sms,
        value: "+44 7700 900000".into(),
        use_type: Some(ContactPointUse::Mobile),
    });

    // 3 addresses — first becomes `address`, rest become `previous_addresses`.
    for (i, city) in ["London", "Manchester", "Edinburgh"].iter().enumerate() {
        p.addresses.push(Address {
            use_type: Some(AddressUse::Home),
            line1: Some(format!("{} Old Lane", i + 1)),
            line2: None,
            city: Some((*city).into()),
            state: Some("ENG".into()),
            postal_code: Some(format!("AB{} 1CD", i + 1)),
            country: Some("GB".into()),
        });
    }

    // 1 passport — exercises the document → PassportBook path.
    p.documents.push(IdentityDocument {
        document_type: DocumentType::Passport,
        number: "X12345678".into(),
        issuing_country: Some("US".into()),
        issuing_authority: None,
        issue_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        expiry_date: Some(NaiveDate::from_ymd_opt(2030, 1, 1).unwrap()),
        verified: true,
    });
    p
}

/// A matching engine with the default configuration.
fn engine() -> MatchingEngine {
    MatchingEngine::new(MatchConfig::default())
}

// =============================================================================
// Group 1: adapter projection cost (minimal vs rich record)
// =============================================================================

/// Benchmark the adapter projection alone, for minimal vs rich records.
fn bench_adapter_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_adapter_only");
    let minimal = minimal_person();
    let rich = rich_person();

    group.bench_function("minimal", |b| {
        b.iter(|| {
            let m = to_matcher_person(black_box(&minimal));
            black_box(m);
        });
    });
    group.bench_function("rich", |b| {
        b.iter(|| {
            let m = to_matcher_person(black_box(&rich));
            black_box(m);
        });
    });

    group.finish();
}

// =============================================================================
// Group 2: end-to-end bridge — adapter + engine
// =============================================================================

/// Benchmark the full bridge (adapter + engine) on clone pairs.
fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_end_to_end");
    let engine = engine();
    let minimal_a = minimal_person();
    let minimal_b = minimal_a.clone();
    let rich_a = rich_person();
    let rich_b = rich_a.clone();

    group.bench_function("minimal_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_persons(
                &to_matcher_person(black_box(&minimal_a)),
                &to_matcher_person(black_box(&minimal_b)),
            );
            black_box(r);
        });
    });
    group.bench_function("rich_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_persons(
                &to_matcher_person(black_box(&rich_a)),
                &to_matcher_person(black_box(&rich_b)),
            );
            black_box(r);
        });
    });

    group.finish();
}

// =============================================================================
// Group 3: one-vs-many — the realistic dedup-on-create path
// =============================================================================

/// Benchmark one query against 10/50/100 candidates (the dedup path).
fn bench_one_to_many(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_one_to_many");
    let engine = engine();
    let query = rich_person();

    // 100 candidates — varied so the matcher can't trivially short-circuit.
    let candidates: Vec<Person> = (0..100)
        .map(|i| {
            let mut p = rich_person();
            p.id = Uuid::new_v4();
            p.name.given[0] = format!("Variant{}", i);
            p
        })
        .collect();

    for &n in &[10usize, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let qm = to_matcher_person(black_box(&query));
                let mut best = 0.0_f64;
                for c in candidates.iter().take(n) {
                    let cm = to_matcher_person(c);
                    let r = engine.match_persons(&qm, &cm);
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
