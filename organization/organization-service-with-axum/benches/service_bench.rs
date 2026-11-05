#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `organization-service` pure request-path
//! work: payload validation, record merge, and the Tantivy retrieval
//! that backs duplicate detection.
//!
//! Run with `cargo bench`. Nothing here touches Postgres — these are the
//! CPU-bound halves of a request, which is exactly the part a database
//! benchmark would hide behind I/O.
//!
//! What the groups are for:
//!
//! - **`validation`** — runs on every create and update. The
//!   `oversized_arrays` case exercises the SEC-M1 input caps: rejecting
//!   an abusive payload must be *cheap*, or the caps are not doing the
//!   job they exist for.
//! - **`merge`** — a whole-record fold; the scaling case shows the cost
//!   sits in the collections it unions.
//! - **`search`** — `index_one_document` is what every create, update,
//!   and merge pays synchronously; `duplicate_candidates` is what a
//!   duplicate check actually calls.

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use organization_matcher::{IdentifierScheme, OrgIdentifier, Organization, PostalAddress};
use organization_service::{merge, search::SearchEngine, validation};
use uuid::Uuid;

fn address(street: &str, locality: &str, postcode: &str) -> PostalAddress {
    PostalAddress {
        street_address: Some(street.into()),
        locality: Some(locality.into()),
        region: Some("Greater London".into()),
        postal_code: Some(postcode.into()),
        country: Some("GB".into()),
    }
}

/// A fully populated organization — every validated field present, so
/// validation runs its whole rule set.
fn populated() -> Organization {
    let mut o = Organization::new("Acme Health Services");
    o.legal_name = Some("Acme Health Services Limited".into());
    o.alternate_names = vec!["AHS".into(), "Acme Health".into()];
    o.url = Some("https://www.acmehealth.example".into());
    o.same_as = vec!["https://www.wikidata.example/Q123".into()];
    o.address = Some(address("10 High Street", "London", "EC1A 1AA"));
    o.jurisdiction = Some("GB".into());
    o.founding_date = Some("1998-04-01".into());
    o.telephone = Some("+44 20 7946 0100".into());
    o.email = Some("contact@acmehealth.example".into());
    o.keywords = vec!["healthcare".into(), "clinics".into()];
    o.identifiers = vec![OrgIdentifier {
        scheme: IdentifierScheme::Lei,
        value: "213800WSGIIZCXF1P572".into(),
    }];
    o
}

/// A payload violating several rules at once — the controller collects
/// every problem in one pass, so this is the realistic bad-request shape.
fn invalid() -> Organization {
    let mut o = populated();
    o.name = "   ".into();
    o.founding_date = Some("1998-13-45".into());
    o.keywords = vec![String::new(), "healthcare".into()];
    o.identifiers = vec![OrgIdentifier {
        scheme: IdentifierScheme::Lei,
        value: String::new(),
    }];
    o
}

/// Deliberately over the SEC-M1 caps: the shape an attacker sends to buy
/// O(n·m) scoring work with one cheap request.
fn oversized() -> Organization {
    let mut o = populated();
    o.name = "x".repeat(64 * 1024);
    o.alternate_names = (0..10_000).map(|i| format!("alias-{i}")).collect();
    o.keywords = (0..10_000).map(|i| format!("keyword-{i}")).collect();
    o
}

/// The `idx`-th distinct organization, for the index and merge fixtures.
fn variant(idx: usize) -> Organization {
    let stems = ["Acme", "Beta", "Caledonian", "Delta", "Eastern"];
    let cities = ["London", "Manchester", "Cardiff", "Glasgow", "Belfast"];
    let mut o = Organization::new(format!(
        "{} Health Services {idx}",
        stems[idx % stems.len()]
    ));
    o.legal_name = Some(format!("{} Health Limited", stems[idx % stems.len()]));
    o.address = Some(address(
        &format!("{} High Street", idx % 200 + 1),
        cities[idx % cities.len()],
        "EC1A 1AA",
    ));
    o.jurisdiction = Some("GB".into());
    o.keywords = vec!["healthcare".into(), format!("sector-{}", idx % 30)];
    o
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
    group.finish();
}

fn bench_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("merge");
    let main = populated();
    let duplicate = variant(7);
    group.bench_function("typical_pair", |b| {
        b.iter(|| merge::merge_orgs(black_box(&main), black_box(&duplicate)));
    });

    // Collection union is where merge's cost lives, so scale that.
    let mut big_main = populated();
    big_main.alternate_names = (0..500).map(|i| format!("alias-{i}")).collect();
    big_main.keywords = (0..500).map(|i| format!("keyword-{i}")).collect();
    let mut big_dup = populated();
    big_dup.alternate_names = (250..750).map(|i| format!("alias-{i}")).collect();
    big_dup.keywords = (250..750).map(|i| format!("keyword-{i}")).collect();
    group.bench_function("many_names_half_overlapping", |b| {
        b.iter(|| merge::merge_orgs(black_box(&big_main), black_box(&big_dup)));
    });
    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("temp dir");
    let engine = SearchEngine::new(dir.path()).expect("index opens");
    for i in 0..500 {
        engine
            .index_organization(Uuid::new_v4(), &variant(i))
            .expect("index write");
    }

    let mut group = c.benchmark_group("search");
    // Its own index, deliberately: measured against the populated one
    // the number would be dominated by accumulated segment state rather
    // than by what a write costs.
    let write_dir = tempfile::tempdir().expect("temp dir");
    let write_engine = SearchEngine::new(write_dir.path()).expect("index opens");
    group.bench_function("index_one_document", |b| {
        let org = populated();
        b.iter(|| {
            write_engine
                .index_organization(black_box(Uuid::new_v4()), black_box(&org))
                .expect("index write");
        });
    });
    group.bench_function("exact", |b| {
        b.iter(|| engine.search(black_box("Acme Health"), 20).expect("search"));
    });
    group.bench_function("fuzzy", |b| {
        b.iter(|| {
            engine
                .fuzzy_search(black_box("Acme Helth"), 20)
                .expect("search")
        });
    });
    group.bench_function("phonetic", |b| {
        b.iter(|| {
            engine
                .phonetic_search(black_box("Akme Helth"), 20)
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
