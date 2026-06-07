//! Criterion benchmarks proxying the database-write preparation cost.
//!
//! Without a live database, these measure the boundary work that precedes an
//! insert: constructing a place and either validating it or normalizing it.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use place_service::models::address::PostalAddress;
use place_service::models::geo::GeoCoordinates;
use place_service::models::place::Place;
use place_service::validation::{normalize_place, validate_place};

/// Benchmark constructing and validating a place (the create-time check).
fn bench_place_create_validate(c: &mut Criterion) {
    c.bench_function("place_create_and_validate", |b| {
        b.iter(|| {
            let mut place = Place::new("Test Place");
            place.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
            place.address = Some(PostalAddress {
                street_address: Some("123 Main St".into()),
                address_locality: Some("New York".into()),
                address_region: Some("NY".into()),
                address_country: Some("US".into()),
                postal_code: Some("10001".into()),
            });
            let errors = validate_place(black_box(&place));
            black_box(errors);
        })
    });
}

/// Benchmark constructing and normalizing a place (the create-time cleanup).
fn bench_place_create_normalize(c: &mut Criterion) {
    c.bench_function("place_create_and_normalize", |b| {
        b.iter_batched(
            || {
                let mut place = Place::new("Test Place");
                place.address = Some(PostalAddress {
                    street_address: Some("123 main st".into()),
                    address_locality: Some("new york".into()),
                    address_region: Some("ny".into()),
                    address_country: Some("us".into()),
                    postal_code: Some("10001".into()),
                });
                place
            },
            |mut place| normalize_place(black_box(&mut place)),
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_place_create_validate, bench_place_create_normalize);
criterion_main!(benches);
