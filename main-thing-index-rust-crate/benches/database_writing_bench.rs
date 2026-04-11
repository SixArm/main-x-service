use criterion::{black_box, criterion_group, criterion_main, Criterion};
use main_thing_index::models::address::PostalAddress;
use main_thing_index::models::geo::GeoCoordinates;
use main_thing_index::models::thing::Thing;
use main_thing_index::validation::{normalize_thing, validate_thing};

fn bench_thing_create_validate(c: &mut Criterion) {
    c.bench_function("thing_create_and_validate", |b| {
        b.iter(|| {
            let mut thing = Thing::new("Test Thing");
            thing.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
            thing.address = Some(PostalAddress {
                street_address: Some("123 Main St".into()),
                address_locality: Some("New York".into()),
                address_region: Some("NY".into()),
                address_country: Some("US".into()),
                postal_code: Some("10001".into()),
            });
            let errors = validate_thing(black_box(&thing));
            black_box(errors);
        })
    });
}

fn bench_thing_create_normalize(c: &mut Criterion) {
    c.bench_function("thing_create_and_normalize", |b| {
        b.iter_batched(
            || {
                let mut thing = Thing::new("Test Thing");
                thing.address = Some(PostalAddress {
                    street_address: Some("123 main st".into()),
                    address_locality: Some("new york".into()),
                    address_region: Some("ny".into()),
                    address_country: Some("us".into()),
                    postal_code: Some("10001".into()),
                });
                thing
            },
            |mut thing| normalize_thing(black_box(&mut thing)),
            criterion::BatchSize::SmallInput,
        )
    });
}

criterion_group!(benches, bench_thing_create_validate, bench_thing_create_normalize);
criterion_main!(benches);
