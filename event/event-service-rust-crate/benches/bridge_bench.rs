#![warn(clippy::pedantic)]

//! Bridge-path benchmarks: service-side `Event` → `to_matcher_event` →
//! `event_matcher::MatchingEngine::match_events`.
//!
//! Measures the cost of the adapter projection and the end-to-end bridge so
//! any future change to the adapter or the matcher's hot path shows up as a
//! clear regression. Three groups:
//!
//! - `bridge_adapter_only` — `to_matcher_event` on minimal and rich records.
//! - `bridge_end_to_end`   — adapter + engine call (the realistic dedup path).
//! - `bridge_one_to_many`  — single query vs. 100 candidates.

use chrono::{DateTime, TimeZone, Utc};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

use event_service::matching::adapter::to_matcher_event;
use event_service::matching::matcher_lib::{MatchConfig, MatchingEngine};
use event_service::models::{
    Address, AddressUse, Event, EventStatus, EventType, Location, Party, PartyKind, Place,
    identifier::{Identifier, IdentifierType},
};

/// A fixed top-of-hour UTC start for deterministic fixtures.
fn start_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap()
}

// -------- fixtures -----------------------------------------------------------

/// A bare event with only the required name + start fields — the
/// cheapest possible adapter input.
fn minimal_event() -> Event {
    Event::new("Test Event", start_at())
}

/// A fully-populated conference (end date, location, organizer,
/// identifier) — the realistic adapter input.
fn rich_event() -> Event {
    let mut e = Event::new("Annual Conference", start_at());
    e.end_date = Some(start_at() + chrono::Duration::hours(8));
    e.event_type = EventType::Conference;
    e.event_status = EventStatus::Scheduled;
    e.location = vec![Location::Place(Place {
        id: None,
        name: "Greek Theatre".into(),
        address: Some(Address {
            use_type: Some(AddressUse::Work),
            line1: None,
            line2: None,
            city: Some("Berkeley".into()),
            state: Some("CA".into()),
            postal_code: Some("94720".into()),
            country: Some("US".into()),
        }),
        latitude: Some(37.873),
        longitude: Some(-122.255),
        url: None,
    })];
    e.organizers = vec![Party {
        kind: PartyKind::Organization,
        id: None,
        name: "Cal Performances".into(),
        email: None,
        url: None,
    }];
    e.identifiers.push(Identifier {
        use_type: None,
        identifier_type: IdentifierType::ExternalRef,
        system: "https://eventbrite.com/event-id".into(),
        value: "123456789".into(),
        assigner: None,
    });
    e
}

/// A matching engine with the default configuration.
fn engine() -> MatchingEngine {
    MatchingEngine::new(MatchConfig::default())
}

// =============================================================================
// Group 1: adapter projection cost (minimal vs rich record)
// =============================================================================

/// Benchmark `to_matcher_event` projection alone, on minimal and rich
/// records.
fn bench_adapter_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_adapter_only");
    let minimal = minimal_event();
    let rich = rich_event();

    group.bench_function("minimal", |b| {
        b.iter(|| {
            let m = to_matcher_event(black_box(&minimal));
            black_box(m);
        });
    });
    group.bench_function("rich", |b| {
        b.iter(|| {
            let m = to_matcher_event(black_box(&rich));
            black_box(m);
        });
    });

    group.finish();
}

// =============================================================================
// Group 2: end-to-end bridge — adapter + engine
// =============================================================================

/// Benchmark the full bridge path: adapter projection of both sides
/// plus the engine `match_events` call.
fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_end_to_end");
    let engine = engine();
    let minimal_a = minimal_event();
    let minimal_b = minimal_a.clone();
    let rich_a = rich_event();
    let rich_b = rich_a.clone();

    group.bench_function("minimal_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_events(
                &to_matcher_event(black_box(&minimal_a)),
                &to_matcher_event(black_box(&minimal_b)),
            );
            black_box(r);
        });
    });
    group.bench_function("rich_clone_match", |b| {
        b.iter(|| {
            let r = engine.match_events(
                &to_matcher_event(black_box(&rich_a)),
                &to_matcher_event(black_box(&rich_b)),
            );
            black_box(r);
        });
    });

    group.finish();
}

// =============================================================================
// Group 3: one-vs-many — the realistic dedup-on-create path
// =============================================================================

/// Benchmark scoring one query against 10 / 50 / 100 candidates — the
/// realistic dedup-on-create fan-out.
fn bench_one_to_many(c: &mut Criterion) {
    let mut group = c.benchmark_group("bridge_one_to_many");
    let engine = engine();
    let query = rich_event();

    // 100 candidates — varied so the matcher can't trivially short-circuit.
    let candidates: Vec<Event> = (0..100)
        .map(|i| {
            let mut p = rich_event();
            p.name = format!("Variant{i} Conference");
            p
        })
        .collect();

    for &n in &[10usize, 50, 100] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let qm = to_matcher_event(black_box(&query));
                let mut best = 0.0_f64;
                for c in candidates.iter().take(n) {
                    let cm = to_matcher_event(c);
                    let r = engine.match_events(&qm, &cm);
                    if r.score > best {
                        best = r.score;
                    }
                }
                black_box(best);
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_adapter_only,
    bench_end_to_end,
    bench_one_to_many,
);
criterion_main!(benches);
