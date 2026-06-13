#![warn(clippy::pedantic)]

//! Criterion benchmarks for validation and normalization: a minimal record,
//! a fully-populated record, and in-place normalization.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;
use thing_service::validation::{normalize_thing, validate_thing};

/// Benchmark `validate_thing` on a name-only record.
fn bench_validate_simple(c: &mut Criterion) {
    let thing = Thing::new("Simple Thing");
    c.bench_function("validate_simple_thing", |b| {
        b.iter(|| validate_thing(black_box(&thing)))
    });
}

/// Benchmark `validate_thing` on a fully-populated record (URLs + identifiers).
fn bench_validate_full(c: &mut Criterion) {
    let mut thing = Thing::new("Full Thing");
    thing.description = Some("A description.".into());
    thing.url = Some("https://example.com".into());
    thing.additional_type = Some("https://schema.org/Book".into());
    thing.identifiers = vec![
        ThingIdentifier::isbn("9780141439518"),
        ThingIdentifier::doi("10.1000/xyz123"),
    ];
    thing.same_as = vec!["https://www.wikidata.org/wiki/Q170583".into()];

    c.bench_function("validate_full_thing", |b| {
        b.iter(|| validate_thing(black_box(&thing)))
    });
}

/// Benchmark in-place `normalize_thing` (trim, scheme-lowercase, dedupe).
fn bench_normalize(c: &mut Criterion) {
    c.bench_function("normalize_thing", |b| {
        b.iter_batched(
            || {
                let mut thing = Thing::new("  test thing  ");
                thing.url = Some("HTTPS://Example.com/Path".into());
                thing.alternate_names = vec!["A".into(), "A".into(), "B".into()];
                thing.same_as = vec!["https://example.com".into(), "https://example.com".into()];
                thing
            },
            |mut thing| normalize_thing(black_box(&mut thing)),
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    benches,
    bench_validate_simple,
    bench_validate_full,
    bench_normalize
);
criterion_main!(benches);
