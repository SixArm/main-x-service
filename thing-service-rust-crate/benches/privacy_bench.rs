//! Criterion benchmarks for the privacy operations: masking (rich and
//! minimal records) and GDPR export (single and 100-record batch).

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;
use thing_service::privacy::{gdpr_export, mask_thing};

/// Benchmark `mask_thing` on a record with owner and identifiers.
fn bench_mask_thing(c: &mut Criterion) {
    let mut thing = Thing::new("Benchmark Thing");
    thing.owner = Some("Owner Name".into());
    thing.identifiers = vec![
        ThingIdentifier::isbn("9780141439518"),
        ThingIdentifier::serial_number("SN-1234567890"),
    ];

    c.bench_function("mask_thing", |b| {
        b.iter(|| mask_thing(black_box(&thing)))
    });
}

/// Benchmark `mask_thing` on a name-only record (no sensitive fields).
fn bench_mask_thing_minimal(c: &mut Criterion) {
    let thing = Thing::new("Minimal Thing");
    c.bench_function("mask_thing_minimal", |b| {
        b.iter(|| mask_thing(black_box(&thing)))
    });
}

/// Benchmark `gdpr_export` on a single record.
fn bench_gdpr_export(c: &mut Criterion) {
    let mut thing = Thing::new("Export Thing");
    thing.description = Some("A test thing for benchmarking".into());
    thing.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    thing.same_as = vec!["https://www.wikidata.org/wiki/Q170583".into()];

    c.bench_function("gdpr_export", |b| {
        b.iter(|| gdpr_export(black_box(&thing)))
    });
}

/// Benchmark `gdpr_export` across a batch of 100 records.
fn bench_gdpr_export_batch(c: &mut Criterion) {
    let things: Vec<Thing> = (0..100)
        .map(|i| {
            let mut t = Thing::new(&format!("Thing {i}"));
            t.owner = Some(format!("Owner {i}"));
            t
        })
        .collect();

    c.bench_function("gdpr_export_batch_100", |b| {
        b.iter(|| {
            for t in &things {
                gdpr_export(black_box(t));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_mask_thing,
    bench_mask_thing_minimal,
    bench_gdpr_export,
    bench_gdpr_export_batch,
);
criterion_main!(benches);
