#![warn(clippy::pedantic)]

//! Criterion benchmarks for the matching hot path: name / url / Soundex
//! component functions, a full `compute_match`, and a 100-candidate batch.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use thing_service::matching::name::name_similarity;
use thing_service::matching::phonetic::soundex;
use thing_service::matching::scoring::{MatchWeights, compute_match};
use thing_service::matching::url::url_similarity;
use thing_service::models::identifier::ThingIdentifier;
use thing_service::models::thing::Thing;

/// Benchmark `name_similarity` for exact, fuzzy, and different name pairs.
fn bench_name_similarity(c: &mut Criterion) {
    c.bench_function("name_similarity_exact", |b| {
        b.iter(|| {
            name_similarity(
                black_box("Pride and Prejudice"),
                black_box("Pride and Prejudice"),
            )
        })
    });
    c.bench_function("name_similarity_fuzzy", |b| {
        b.iter(|| {
            name_similarity(
                black_box("Pride and Prejudice"),
                black_box("Prde and Prejudice"),
            )
        })
    });
    c.bench_function("name_similarity_different", |b| {
        b.iter(|| name_similarity(black_box("Pride and Prejudice"), black_box("War and Peace")))
    });
}

/// Benchmark `url_similarity` for identical and different URLs.
fn bench_url_similarity(c: &mut Criterion) {
    c.bench_function("url_similarity_identical", |bench| {
        bench.iter(|| {
            url_similarity(
                black_box("https://en.wikipedia.org/wiki/Pride_and_Prejudice"),
                black_box("https://en.wikipedia.org/wiki/Pride_and_Prejudice"),
            )
        })
    });
    c.bench_function("url_similarity_different", |bench| {
        bench.iter(|| {
            url_similarity(
                black_box("https://en.wikipedia.org/wiki/Pride_and_Prejudice"),
                black_box("https://www.rust-lang.org"),
            )
        })
    });
}

/// Benchmark `soundex` on a short and a long word.
fn bench_soundex(c: &mut Criterion) {
    c.bench_function("soundex_short", |b| b.iter(|| soundex(black_box("Book"))));
    c.bench_function("soundex_long", |b| {
        b.iter(|| soundex(black_box("Springfield")))
    });
}

/// Benchmark a full `compute_match` over a multi-component record pair.
fn bench_full_match(c: &mut Criterion) {
    let mut a = Thing::new("Pride and Prejudice");
    a.description = Some("A novel by Jane Austen.".into());
    a.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    a.identifiers = vec![ThingIdentifier::isbn("9780141439518")];
    a.same_as = vec!["https://www.wikidata.org/wiki/Q170583".into()];

    let mut b = Thing::new("Prde and Prejudice");
    b.description = Some("A novel by Jane Austen.".into());
    b.url = Some("https://en.wikipedia.org/wiki/Pride_and_Prejudice".into());
    b.same_as = vec!["https://www.wikidata.org/wiki/Q170583".into()];

    let weights = MatchWeights::default();

    c.bench_function("full_thing_match", |bench| {
        bench.iter(|| compute_match(black_box(&a), black_box(&b), black_box(&weights)))
    });
}

/// Benchmark matching one target against 100 candidates.
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
    bench_url_similarity,
    bench_soundex,
    bench_full_match,
    bench_batch_matching,
);
criterion_main!(benches);
