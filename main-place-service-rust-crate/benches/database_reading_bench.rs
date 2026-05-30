use criterion::{black_box, criterion_group, criterion_main, Criterion};
use main_place_service::models::place::Place;

fn bench_place_construction(c: &mut Criterion) {
    c.bench_function("place_construction", |b| {
        b.iter(|| Place::new(black_box("Test Place")))
    });
}

fn bench_place_batch_construction(c: &mut Criterion) {
    c.bench_function("place_batch_construction_100", |b| {
        b.iter(|| {
            let places: Vec<Place> = (0..100)
                .map(|i| Place::new(&format!("Place {i}")))
                .collect();
            black_box(places);
        })
    });
}

criterion_group!(benches, bench_place_construction, bench_place_batch_construction);
criterion_main!(benches);
