use criterion::{black_box, criterion_group, criterion_main, Criterion};
use main_thing_index::models::geo::GeoCoordinates;
use main_thing_index::models::thing::Thing;
use main_thing_index::privacy::{gdpr_export, mask_thing};

fn bench_mask_thing(c: &mut Criterion) {
    let mut thing = Thing::new("Benchmark Thing");
    thing.telephone = Some("+1-555-867-5309".into());
    thing.fax_number = Some("+1-555-123-4567".into());
    thing.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));

    c.bench_function("mask_thing", |b| {
        b.iter(|| mask_thing(black_box(&thing)))
    });
}

fn bench_mask_thing_minimal(c: &mut Criterion) {
    let thing = Thing::new("Minimal Thing");
    c.bench_function("mask_thing_minimal", |b| {
        b.iter(|| mask_thing(black_box(&thing)))
    });
}

fn bench_gdpr_export(c: &mut Criterion) {
    let mut thing = Thing::new("Export Thing");
    thing.telephone = Some("+1-555-867-5309".into());
    thing.description = Some("A test thing for benchmarking".into());
    thing.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    c.bench_function("gdpr_export", |b| {
        b.iter(|| gdpr_export(black_box(&thing)))
    });
}

fn bench_gdpr_export_batch(c: &mut Criterion) {
    let things: Vec<Thing> = (0..100)
        .map(|i| {
            let mut p = Thing::new(&format!("Thing {i}"));
            p.telephone = Some(format!("+1-555-{i:04}"));
            p.geo = Some(GeoCoordinates::new(40.0 + i as f64 * 0.01, -74.0));
            p
        })
        .collect();

    c.bench_function("gdpr_export_batch_100", |b| {
        b.iter(|| {
            for p in &things {
                gdpr_export(black_box(p));
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
