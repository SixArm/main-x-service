//! Benchmarks for the Tantivy-backed event search engine.
//!
//! Measures single-event indexing throughput and full-text / fuzzy
//! query latency over a 500-document seeded index. Run with
//! `cargo bench`.

use chrono::{TimeZone, Utc};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

use event_service::models::Event;
use event_service::search::SearchEngine;

/// A pool of representative event titles cycled through when seeding
/// the benchmark index.
const TITLES: &[&str] = &[
    "Annual Conference",
    "Hackathon",
    "Music Festival",
    "Book Signing",
    "Tech Talk",
    "Comedy Night",
    "Art Exhibition",
    "Film Screening",
    "Sports Match",
    "Workshop",
];

/// Build an event with the given title at the given hour on a fixed
/// date.
fn make_event(title: &str, hour: u32) -> Event {
    let when = Utc.with_ymd_and_hms(2026, 3, 1, hour, 0, 0).unwrap();
    Event::new(title, when)
}

/// Build a search engine over a fresh tempdir seeded with `n` indexed
/// events. The [`TempDir`] is returned so the caller keeps it alive
/// for the duration of the benchmark.
fn build_seeded_engine(n: usize) -> (TempDir, SearchEngine) {
    let tmp = TempDir::new().unwrap();
    let engine = SearchEngine::new(tmp.path()).unwrap();
    let events: Vec<Event> = (0..n)
        .map(|i| make_event(TITLES[i % TITLES.len()], (9 + (i as u32 % 12)) % 24))
        .collect();
    engine.index_events(&events).unwrap();
    engine.reload().unwrap();
    (tmp, engine)
}

/// Benchmark indexing a single event into a fresh index.
fn bench_index_single_event(c: &mut Criterion) {
    let tmp = TempDir::new().unwrap();
    let engine = SearchEngine::new(tmp.path()).unwrap();
    let event = make_event("Annual Conference", 9);
    c.bench_function("index_single_event", |b| {
        b.iter(|| black_box(engine.index_event(black_box(&event)).unwrap()));
    });
}

/// Benchmark a full-text query over a 500-document index.
fn bench_full_text_search(c: &mut Criterion) {
    let (_tmp, engine) = build_seeded_engine(500);
    c.bench_function("full_text_search_500_docs", |b| {
        b.iter(|| black_box(engine.search(black_box("Conference"), 10).unwrap()));
    });
}

/// Benchmark a fuzzy (typo-tolerant) query over a 500-document index.
fn bench_fuzzy_search(c: &mut Criterion) {
    let (_tmp, engine) = build_seeded_engine(500);
    c.bench_function("fuzzy_search_500_docs", |b| {
        b.iter(|| black_box(engine.fuzzy_search(black_box("Hakathon"), 10).unwrap()));
    });
}

criterion_group!(
    benches,
    bench_index_single_event,
    bench_full_text_search,
    bench_fuzzy_search,
);
criterion_main!(benches);
