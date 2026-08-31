#![warn(clippy::pedantic)]

//! Criterion benchmarks for `entity-ref`.
//!
//! Run with `cargo bench`. These are tiny operations, but they run at
//! graph scale: the link-graph aggregator parses two `EntityRef`s per
//! edge row, so a `neighbors` query over ten thousand edges parses
//! twenty thousand refs before it does anything else. A cost that is
//! irrelevant per call is not irrelevant per query.
//!
//! What the numbers are for:
//!
//! - **Parse versus render.** `Display` allocates a `String`; parsing
//!   does not have to. If rendering is the cheaper half, a hot path that
//!   round-trips is doing avoidable work.
//! - **The reject path.** `from_token` is a linear scan over the closed
//!   registry, so an unknown type walks the whole list. That is fine at
//!   ten entries and worth knowing before it is fifty.
//! - **serde versus direct parse.** The gap is what the JSON layer costs
//!   over `FromStr`, which is the choice a bulk import makes per row.

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use entity_ref::{EdgeKind, EntityRef, EntityType};
use std::hint::black_box;
use std::str::FromStr as _;

const PERSON_REF: &str = "person:0c4f1e2a-0000-4000-8000-000000000000";
/// The last type in the registry, so `from_token` walks the whole list —
/// the worst case for a *successful* lookup.
const CARE_PATHWAY_REF: &str = "care_pathway:7b3d9911-0000-4000-8000-000000000000";

/// A batch of refs shaped like an edge read-model page: a mix of types,
/// distinct ids.
fn ref_batch(n: usize) -> Vec<String> {
    let types = [
        "person",
        "worker",
        "organization",
        "case",
        "courseinstance",
        "care_pathway",
    ];
    (0..n)
        .map(|i| {
            format!(
                "{}:{:08x}-0000-4000-8000-{:012x}",
                types[i % types.len()],
                i,
                i
            )
        })
        .collect()
}

/// Parsing one ref: the common success case, the worst-case successful
/// registry lookup, and the two reject paths (unknown type, bad uuid).
fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    group.bench_function("first_registry_entry", |b| {
        b.iter(|| EntityRef::from_str(black_box(PERSON_REF)).expect("valid"));
    });
    group.bench_function("last_registry_entry", |b| {
        b.iter(|| EntityRef::from_str(black_box(CARE_PATHWAY_REF)).expect("valid"));
    });
    group.bench_function("unknown_type", |b| {
        b.iter(|| EntityRef::from_str(black_box("dragon:0c4f1e2a-0000-4000-8000-000000000000")));
    });
    group.bench_function("bad_uuid", |b| {
        b.iter(|| EntityRef::from_str(black_box("person:not-a-uuid")));
    });
    group.bench_function("no_separator", |b| {
        b.iter(|| EntityRef::from_str(black_box("person")));
    });
    group.finish();
}

/// Rendering back to the wire form, and the full round-trip a
/// store-then-reload performs.
fn bench_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    let parsed = EntityRef::from_str(PERSON_REF).expect("valid");

    group.bench_function("display", |b| {
        b.iter(|| black_box(&parsed).to_string());
    });
    group.bench_function("round_trip", |b| {
        b.iter(|| EntityRef::from_str(&black_box(&parsed).to_string()).expect("round-trips"));
    });
    group.finish();
}

/// The serde bridge — how a request body and a `TEXT` column actually
/// arrive — against the direct `FromStr` above.
fn bench_serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde");
    let json = format!("\"{PERSON_REF}\"");
    let parsed = EntityRef::from_str(PERSON_REF).expect("valid");

    group.bench_function("deserialize", |b| {
        b.iter(|| serde_json::from_str::<EntityRef>(black_box(&json)).expect("valid"));
    });
    group.bench_function("serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&parsed)).expect("serializes"));
    });
    group.finish();
}

/// A page of edge rows: what the aggregator pays to turn stored strings
/// back into refs. `Throughput::Elements` reports per-ref cost.
fn bench_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_batch");
    for &n in &[100usize, 10_000] {
        let batch = ref_batch(n);
        group.throughput(Throughput::Elements(n as u64));
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(n),
            &batch,
            |b, batch| {
                // Collect rather than `count()`: an aggregator keeps the
                // parsed refs, and a `count()` invites the optimiser (and
                // clippy) to observe that the parse result is discarded.
                b.iter(|| {
                    batch
                        .iter()
                        .map(|s| EntityRef::from_str(s).expect("valid"))
                        .collect::<Vec<_>>()
                });
            },
        );
    }
    group.finish();
}

/// The edge-kind registry lookups the aggregator runs per row alongside
/// the refs.
fn bench_edge_kind(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_kind");
    group.bench_function("from_token_hit", |b| {
        b.iter(|| EdgeKind::from_token(black_box("subject_of")).expect("known"));
    });
    group.bench_function("from_token_miss", |b| {
        b.iter(|| EdgeKind::from_token(black_box("not_a_kind")));
    });
    group.bench_function("permits", |b| {
        b.iter(|| {
            EdgeKind::SubjectOf.permits(black_box(EntityType::Case), black_box(EntityType::Person))
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_render,
    bench_serde,
    bench_batch,
    bench_edge_kind
);
criterion_main!(benches);
