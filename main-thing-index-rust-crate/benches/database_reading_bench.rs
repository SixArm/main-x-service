use criterion::{black_box, criterion_group, criterion_main, Criterion};
use main_thing_index::models::thing::Thing;

fn bench_thing_construction(c: &mut Criterion) {
    c.bench_function("thing_construction", |b| {
        b.iter(|| Thing::new(black_box("Test Thing")))
    });
}

fn bench_thing_batch_construction(c: &mut Criterion) {
    c.bench_function("thing_batch_construction_100", |b| {
        b.iter(|| {
            let things: Vec<Thing> = (0..100)
                .map(|i| Thing::new(&format!("Thing {i}")))
                .collect();
            black_box(things);
        })
    });
}

criterion_group!(benches, bench_thing_construction, bench_thing_batch_construction);
criterion_main!(benches);
