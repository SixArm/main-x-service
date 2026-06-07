//! Criterion benchmarks approximating write-side cost: the create-time
//! validate and normalize steps that run before a row is persisted.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;
use thing_service::validation::{normalize_thing, validate_thing};

/// Benchmark the create + validate path.
fn bench_thing_create_validate(c: &mut Criterion) {
    c.bench_function("thing_create_and_validate", |b| {
        b.iter(|| {
            let mut thing = Thing::new("Test Thing");
            thing.url = Some("https://example.com".into());
            thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
            let errors = validate_thing(black_box(&thing));
            black_box(errors);
        })
    });
}

/// Benchmark the create + normalize path.
fn bench_thing_create_normalize(c: &mut Criterion) {
    c.bench_function("thing_create_and_normalize", |b| {
        b.iter_batched(
            || {
                let mut thing = Thing::new("  Test Thing  ");
                thing.url = Some("HTTPS://Example.com/Path".into());
                thing.alternate_names = vec!["A".into(), "A".into(), "B".into()];
                thing
            },
            |mut thing| normalize_thing(black_box(&mut thing)),
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_thing_create_validate, bench_thing_create_normalize);
criterion_main!(benches);
