#![warn(clippy::pedantic)]

//! Criterion benchmarks for the `organization-matcher` crate.
//!
//! Run with `cargo bench`. The harness covers the hot paths a downstream
//! integrator exercises: single-pair probabilistic matching, the
//! deterministic short-circuit, batch ranking, and the cost each config
//! preset adds. Numbers are absolute time per call.
//!
//! The ranking group sets `Throughput::Elements`, so Criterion reports
//! per-candidate cost — which is what makes super-linear scaling visible
//! rather than something you infer from three separate absolute numbers.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use organization_matcher::{
    IdentifierScheme, MatchConfig, MatchingEngine, OrgIdentifier, Organization, PostalAddress,
};
use std::hint::black_box;

/// Terse `OrgIdentifier` constructor.
fn ident(scheme: IdentifierScheme, value: &str) -> OrgIdentifier {
    OrgIdentifier {
        scheme,
        value: value.into(),
    }
}

/// A populated postal address, so the address component does real work
/// rather than short-cutting on a missing field.
fn address(street: &str, locality: &str, postcode: &str) -> PostalAddress {
    PostalAddress {
        street_address: Some(street.into()),
        locality: Some(locality.into()),
        region: Some("Greater London".into()),
        postal_code: Some(postcode.into()),
        country: Some("GB".into()),
    }
}

/// The reference organization: every scored component populated (legal
/// name, alternates, address, url, jurisdiction, founding date, contact,
/// keywords) so a benchmark exercises the whole pipeline.
fn build_reference() -> Organization {
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
    o
}

/// A near-duplicate: the "same organization, different registry" case —
/// a legal-suffix variation, an abbreviated street, a re-formatted
/// phone. Carries **no** shared deterministic identifier, so the full
/// probabilistic path runs.
fn build_near_duplicate() -> Organization {
    let mut o = Organization::new("Acme Health Services Ltd");
    o.legal_name = Some("ACME HEALTH SERVICES LTD.".into());
    o.url = Some("http://acmehealth.example/".into());
    o.address = Some(address("10 High St", "London", "EC1A1AA"));
    o.jurisdiction = Some("GB".into());
    o.founding_date = Some("1998".into());
    o.telephone = Some("020 7946 0100".into());
    o.keywords = vec!["healthcare".into()];
    o
}

/// A clearly different organization, for the common "no match" path.
fn build_unrelated() -> Organization {
    let mut o = Organization::new("Northern Rail Freight");
    o.legal_name = Some("Northern Rail Freight PLC".into());
    o.address = Some(address("99 Dock Road", "Glasgow", "G1 1AA"));
    o.jurisdiction = Some("GB".into());
    o.founding_date = Some("1965-11-30".into());
    o.keywords = vec!["logistics".into()];
    o
}

/// Deterministically synthesise the `idx`-th candidate, so a candidate
/// list of any size is reproducible without randomness.
fn make_candidate(idx: usize) -> Organization {
    let stems = ["Acme", "Beta", "Caledonian", "Delta", "Eastern"];
    let suffixes = ["Limited", "PLC", "LLP", "Ltd", "Group"];
    let cities = ["London", "Manchester", "Cardiff", "Glasgow", "Belfast"];
    let mut o = Organization::new(format!(
        "{} Health Services {}",
        stems[idx % stems.len()],
        idx % 300
    ));
    o.legal_name = Some(format!(
        "{} Health {}",
        stems[idx % stems.len()],
        suffixes[idx % suffixes.len()]
    ));
    o.address = Some(address(
        &format!("{} High Street", idx % 200 + 1),
        cities[idx % cities.len()],
        "EC1A 1AA",
    ));
    o.jurisdiction = Some("GB".into());
    o.keywords = vec!["healthcare".into(), format!("sector-{}", idx % 30)];
    o
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
        b.iter(|| engine.match_organizations(black_box(&reference), black_box(&clone)));
    });
    group.bench_function("fuzzy_near_match", |b| {
        b.iter(|| engine.match_organizations(black_box(&reference), black_box(&near)));
    });
    group.bench_function("unrelated_pair", |b| {
        b.iter(|| engine.match_organizations(black_box(&reference), black_box(&unrelated)));
    });
    group.finish();
}

/// The deterministic paths — the cheapest decisions the engine makes and
/// the ones a duplicate check hits most often: a shared LEI (globally
/// unique) and a jurisdiction-scoped tax id.
fn bench_deterministic(c: &mut Criterion) {
    let mut group = c.benchmark_group("deterministic");
    let engine = MatchingEngine::default_config();

    let mut a = build_reference();
    let mut b = build_unrelated();
    a.identifiers
        .push(ident(IdentifierScheme::Lei, "213800WSGIIZCXF1P572"));
    b.identifiers
        .push(ident(IdentifierScheme::Lei, "213800WSGIIZCXF1P572"));
    group.bench_function("shared_lei", |bencher| {
        bencher.iter(|| engine.match_organizations(black_box(&a), black_box(&b)));
    });

    let mut left = build_reference();
    let mut right = build_near_duplicate();
    left.identifiers
        .push(ident(IdentifierScheme::TaxId, "GB123456789"));
    right
        .identifiers
        .push(ident(IdentifierScheme::TaxId, "GB123456789"));
    group.bench_function("same_jurisdiction_tax_id", |bencher| {
        bencher.iter(|| engine.match_organizations(black_box(&left), black_box(&right)));
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
        let candidates: Vec<Organization> = (0..n).map(make_candidate).collect();
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
            b.iter(|| engine.match_organizations(black_box(&reference), black_box(&near)));
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
