use criterion::{black_box, criterion_group, criterion_main, Criterion};
use thing_service::matching::name::name_similarity;
use thing_service::models::thing::Thing;

fn bench_search_by_name(c: &mut Criterion) {
    let things: Vec<Thing> = (0..100)
        .map(|i| Thing::new(&format!("Thing {i}")))
        .collect();

    c.bench_function("search_by_name_100", |b| {
        b.iter(|| {
            let query = "Thing 42";
            for thing in &things {
                name_similarity(black_box(query), black_box(&thing.name));
            }
        })
    });
}

fn bench_search_by_name_fuzzy(c: &mut Criterion) {
    let things: Vec<Thing> = (0..100)
        .map(|i| Thing::new(&format!("Thing {i}")))
        .collect();

    c.bench_function("search_by_name_fuzzy_100", |b| {
        b.iter(|| {
            let query = "Thng 42";
            for thing in &things {
                name_similarity(black_box(query), black_box(&thing.name));
            }
        })
    });
}

criterion_group!(benches, bench_search_by_name, bench_search_by_name_fuzzy);
criterion_main!(benches);
