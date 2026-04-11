use criterion::{black_box, criterion_group, criterion_main, Criterion};
use main_thing_index::models::address::PostalAddress;
use main_thing_index::models::geo::GeoCoordinates;
use main_thing_index::models::thing::Thing;
use main_thing_index::validation::{normalize_thing, validate_thing};

fn bench_validate_simple(c: &mut Criterion) {
    let thing = Thing::new("Simple Thing");
    c.bench_function("validate_simple_thing", |b| {
        b.iter(|| validate_thing(black_box(&thing)))
    });
}

fn bench_validate_full(c: &mut Criterion) {
    let mut thing = Thing::new("Full Thing");
    thing.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
    thing.url = Some("https://example.com".into());
    thing.telephone = Some("+1-555-0100".into());
    thing.global_location_number = Some("1234567890123".into());
    thing.address = Some(PostalAddress {
        street_address: Some("123 Main St".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10001".into()),
    });

    c.bench_function("validate_full_thing", |b| {
        b.iter(|| validate_thing(black_box(&thing)))
    });
}

fn bench_normalize(c: &mut Criterion) {
    c.bench_function("normalize_thing", |b| {
        b.iter_batched(
            || {
                let mut thing = Thing::new("  test thing  ");
                thing.address = Some(PostalAddress {
                    street_address: Some("123 main st".into()),
                    address_locality: Some("san francisco".into()),
                    address_region: Some("ca".into()),
                    address_country: Some("us".into()),
                    postal_code: Some("94111".into()),
                });
                thing
            },
            |mut thing| normalize_thing(black_box(&mut thing)),
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_validate_simple, bench_validate_full, bench_normalize);
criterion_main!(benches);
