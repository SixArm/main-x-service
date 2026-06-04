//! Search micro-benchmarks. Establishes baselines for indexing one
//! course, exact search, fuzzy search, and the duplicate-detector
//! blocking query against a populated 100-course index.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use tempfile::TempDir;
use uuid::Uuid;

use course_service::models::Course;
use course_service::search::SearchEngine;

fn populated_engine() -> (TempDir, SearchEngine, Course) {
    let dir = TempDir::new().expect("tempdir");
    let engine = SearchEngine::new(dir.path()).expect("engine");
    let provider = Uuid::new_v4();

    let probe = {
        let mut c = Course::new("Introduction to Computer Science");
        c.course_code = Some("CS101".into());
        c.provider_id = Some(provider);
        c
    };
    engine.index_course(&probe).expect("index probe");

    for i in 0..100 {
        let mut c = Course::new(format!("Course Number {i:04}"));
        c.course_code = Some(format!("X{i:03}"));
        c.provider_id = Some(provider);
        engine.index_course(&c).expect("index decoy");
    }
    (dir, engine, probe)
}

fn bench_index_single(c: &mut Criterion) {
    let (_dir, engine, _probe) = populated_engine();
    c.bench_function("search/index_course/single", |b| {
        b.iter(|| {
            let mut course = Course::new(format!("Bench {}", Uuid::new_v4()));
            course.course_code = Some("BENCH".into());
            engine.index_course(black_box(&course)).expect("index");
        });
    });
}

fn bench_search_exact(c: &mut Criterion) {
    let (_dir, engine, _probe) = populated_engine();
    c.bench_function("search/exact/computer_science", |b| {
        b.iter(|| {
            black_box(engine.search(black_box("Computer Science"), 10).expect("search"));
        });
    });
}

fn bench_search_fuzzy(c: &mut Criterion) {
    let (_dir, engine, _probe) = populated_engine();
    c.bench_function("search/fuzzy/typo", |b| {
        b.iter(|| {
            black_box(engine.fuzzy_search(black_box("Computre"), 10).expect("fuzzy"));
        });
    });
}

fn bench_blocker(c: &mut Criterion) {
    let (_dir, engine, probe) = populated_engine();
    c.bench_function("search/search_by_name_and_provider", |b| {
        b.iter(|| {
            black_box(
                engine
                    .search_by_name_and_provider(
                        black_box(&probe.name),
                        black_box(probe.provider_id),
                        50,
                    )
                    .expect("blocker"),
            );
        });
    });
}

criterion_group!(
    search,
    bench_index_single,
    bench_search_exact,
    bench_search_fuzzy,
    bench_blocker
);
criterion_main!(search);
