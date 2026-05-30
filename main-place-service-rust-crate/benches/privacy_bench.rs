use criterion::{black_box, criterion_group, criterion_main, Criterion};
use main_place_service::models::geo::GeoCoordinates;
use main_place_service::models::place::Place;
use main_place_service::privacy::{gdpr_export, mask_place};

fn bench_mask_place(c: &mut Criterion) {
    let mut place = Place::new("Benchmark Place");
    place.telephone = Some("+1-555-867-5309".into());
    place.fax_number = Some("+1-555-123-4567".into());
    place.geo = Some(GeoCoordinates::new(40.78293456, -73.96543210));

    c.bench_function("mask_place", |b| {
        b.iter(|| mask_place(black_box(&place)))
    });
}

fn bench_mask_place_minimal(c: &mut Criterion) {
    let place = Place::new("Minimal Place");
    c.bench_function("mask_place_minimal", |b| {
        b.iter(|| mask_place(black_box(&place)))
    });
}

fn bench_gdpr_export(c: &mut Criterion) {
    let mut place = Place::new("Export Place");
    place.telephone = Some("+1-555-867-5309".into());
    place.description = Some("A test place for benchmarking".into());
    place.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    c.bench_function("gdpr_export", |b| {
        b.iter(|| gdpr_export(black_box(&place)))
    });
}

fn bench_gdpr_export_batch(c: &mut Criterion) {
    let places: Vec<Place> = (0..100)
        .map(|i| {
            let mut p = Place::new(&format!("Place {i}"));
            p.telephone = Some(format!("+1-555-{i:04}"));
            p.geo = Some(GeoCoordinates::new(40.0 + i as f64 * 0.01, -74.0));
            p
        })
        .collect();

    c.bench_function("gdpr_export_batch_100", |b| {
        b.iter(|| {
            for p in &places {
                gdpr_export(black_box(p));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_mask_place,
    bench_mask_place_minimal,
    bench_gdpr_export,
    bench_gdpr_export_batch,
);
criterion_main!(benches);
