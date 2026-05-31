use criterion::{black_box, criterion_group, criterion_main, Criterion};
use place_service::matching::name::name_similarity;
use place_service::models::place::Place;

fn bench_search_by_name(c: &mut Criterion) {
    let places: Vec<Place> = (0..100)
        .map(|i| Place::new(&format!("Place {i}")))
        .collect();

    c.bench_function("search_by_name_100", |b| {
        b.iter(|| {
            let query = "Place 42";
            for place in &places {
                name_similarity(black_box(query), black_box(&place.name));
            }
        })
    });
}

fn bench_search_by_name_fuzzy(c: &mut Criterion) {
    let places: Vec<Place> = (0..100)
        .map(|i| Place::new(&format!("Place {i}")))
        .collect();

    c.bench_function("search_by_name_fuzzy_100", |b| {
        b.iter(|| {
            let query = "Plce 42";
            for place in &places {
                name_similarity(black_box(query), black_box(&place.name));
            }
        })
    });
}

criterion_group!(benches, bench_search_by_name, bench_search_by_name_fuzzy);
criterion_main!(benches);
