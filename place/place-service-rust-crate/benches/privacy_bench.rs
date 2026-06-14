#![warn(clippy::pedantic)]

//! Criterion benchmarks for the privacy layer.
//!
//! Measures `mask_place` (rich vs. minimal record) and `gdpr_export` (single
//! record and a batch of 100), the latter dominated by Serde serialization.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use place_service::models::geo::GeoCoordinates;
use place_service::models::place::Place;
use place_service::privacy::{gdpr_export, mask_place};

/// Benchmark masking a record with phone, fax, and geo set.
fn bench_mask_place(c: &mut Criterion) {
    let mut place = Place::new("Benchmark Place");
    place.telephone = Some("+1-555-867-5309".into());
    place.fax_number = Some("+1-555-123-4567".into());
    place.geo = Some(GeoCoordinates::new(40.782_934_56, -73.965_432_10));

    c.bench_function("mask_place", |b| b.iter(|| mask_place(black_box(&place))));
}

/// Benchmark masking a minimal record (no sensitive fields).
fn bench_mask_place_minimal(c: &mut Criterion) {
    let place = Place::new("Minimal Place");
    c.bench_function("mask_place_minimal", |b| {
        b.iter(|| mask_place(black_box(&place)));
    });
}

/// Benchmark a single-record GDPR JSON export.
fn bench_gdpr_export(c: &mut Criterion) {
    let mut place = Place::new("Export Place");
    place.telephone = Some("+1-555-867-5309".into());
    place.description = Some("A test place for benchmarking".into());
    place.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    c.bench_function("gdpr_export", |b| b.iter(|| gdpr_export(black_box(&place))));
}

/// Benchmark GDPR-exporting a batch of 100 records.
fn bench_gdpr_export_batch(c: &mut Criterion) {
    let places: Vec<Place> = (0..100)
        .map(|i| {
            let mut p = Place::new(&format!("Place {i}"));
            p.telephone = Some(format!("+1-555-{i:04}"));
            p.geo = Some(GeoCoordinates::new(40.0 + f64::from(i) * 0.01, -74.0));
            p
        })
        .collect();

    c.bench_function("gdpr_export_batch_100", |b| {
        b.iter(|| {
            for p in &places {
                black_box(gdpr_export(black_box(p)));
            }
        });
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
