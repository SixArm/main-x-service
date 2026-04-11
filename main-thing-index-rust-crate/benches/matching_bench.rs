use criterion::{black_box, criterion_group, criterion_main, Criterion};
use main_thing_index::matching::name::name_similarity;
use main_thing_index::matching::geo::geo_similarity;
use main_thing_index::matching::phonetic::soundex;
use main_thing_index::matching::scoring::{compute_match, MatchWeights};
use main_thing_index::models::address::PostalAddress;
use main_thing_index::models::geo::GeoCoordinates;
use main_thing_index::models::thing::Thing;
use main_thing_index::models::thing_type::ThingType;

fn bench_name_similarity(c: &mut Criterion) {
    c.bench_function("name_similarity_exact", |b| {
        b.iter(|| name_similarity(black_box("Central Park"), black_box("Central Park")))
    });
    c.bench_function("name_similarity_fuzzy", |b| {
        b.iter(|| name_similarity(black_box("Central Park"), black_box("Centrl Park")))
    });
    c.bench_function("name_similarity_different", |b| {
        b.iter(|| name_similarity(black_box("Central Park"), black_box("Golden Gate Bridge")))
    });
}

fn bench_geo_similarity(c: &mut Criterion) {
    let a = GeoCoordinates::new(40.7829, -73.9654);
    let b = GeoCoordinates::new(48.8584, 2.2945);

    c.bench_function("geo_similarity_close", |bench| {
        let near = GeoCoordinates::new(40.7830, -73.9655);
        bench.iter(|| geo_similarity(black_box(&a), black_box(&near)))
    });
    c.bench_function("geo_similarity_far", |bench| {
        bench.iter(|| geo_similarity(black_box(&a), black_box(&b)))
    });
}

fn bench_soundex(c: &mut Criterion) {
    c.bench_function("soundex_short", |b| {
        b.iter(|| soundex(black_box("Park")))
    });
    c.bench_function("soundex_long", |b| {
        b.iter(|| soundex(black_box("Springfield")))
    });
}

fn bench_full_match(c: &mut Criterion) {
    let mut a = Thing::new("Central Park");
    a.thing_type = Some(ThingType::Park);
    a.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
    a.address = Some(PostalAddress {
        street_address: Some("14 E 60th St".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });

    let mut b = Thing::new("Centrl Park");
    b.thing_type = Some(ThingType::Park);
    b.geo = Some(GeoCoordinates::new(40.7830, -73.9655));
    b.address = Some(PostalAddress {
        street_address: Some("14 East 60th Street".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });

    let weights = MatchWeights::default();

    c.bench_function("full_thing_match", |bench| {
        bench.iter(|| compute_match(black_box(&a), black_box(&b), black_box(&weights)))
    });
}

fn bench_batch_matching(c: &mut Criterion) {
    let target = Thing::new("Target Thing");
    let candidates: Vec<Thing> = (0..100)
        .map(|i| Thing::new(&format!("Candidate Thing {i}")))
        .collect();
    let weights = MatchWeights::default();

    c.bench_function("batch_match_100_candidates", |b| {
        b.iter(|| {
            for c in &candidates {
                compute_match(black_box(&target), black_box(c), black_box(&weights));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_name_similarity,
    bench_geo_similarity,
    bench_soundex,
    bench_full_match,
    bench_batch_matching,
);
criterion_main!(benches);
