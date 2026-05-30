use criterion::{black_box, criterion_group, criterion_main, Criterion};
use main_place_service::models::address::PostalAddress;
use main_place_service::models::geo::GeoCoordinates;
use main_place_service::models::place::Place;
use main_place_service::validation::{normalize_place, validate_place};

fn bench_validate_simple(c: &mut Criterion) {
    let place = Place::new("Simple Place");
    c.bench_function("validate_simple_place", |b| {
        b.iter(|| validate_place(black_box(&place)))
    });
}

fn bench_validate_full(c: &mut Criterion) {
    let mut place = Place::new("Full Place");
    place.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
    place.url = Some("https://example.com".into());
    place.telephone = Some("+1-555-0100".into());
    place.global_location_number = Some("1234567890123".into());
    place.address = Some(PostalAddress {
        street_address: Some("123 Main St".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10001".into()),
    });

    c.bench_function("validate_full_place", |b| {
        b.iter(|| validate_place(black_box(&place)))
    });
}

fn bench_normalize(c: &mut Criterion) {
    c.bench_function("normalize_place", |b| {
        b.iter_batched(
            || {
                let mut place = Place::new("  test place  ");
                place.address = Some(PostalAddress {
                    street_address: Some("123 main st".into()),
                    address_locality: Some("san francisco".into()),
                    address_region: Some("ca".into()),
                    address_country: Some("us".into()),
                    postal_code: Some("94111".into()),
                });
                place
            },
            |mut place| normalize_place(black_box(&mut place)),
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_validate_simple, bench_validate_full, bench_normalize);
criterion_main!(benches);
