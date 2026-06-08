//! Benchmarks for the service-side event matching algorithms.
//!
//! Each Criterion group exercises one component of the matcher hot path
//! (name, time, location, party, identifier), plus the end-to-end
//! probabilistic match against 50 candidates and the Soundex phonetic
//! similarity used as a name-score floor. Run with `cargo bench`.

use jiff::Timestamp;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use event_service::config::MatchingConfig;
use event_service::matching::algorithms::{
    identifier_matching, location_matching, name_matching, party_matching, time_matching,
};
use event_service::matching::phonetic;
use event_service::matching::{EventMatcher, ProbabilisticMatcher};
use event_service::models::*;

/// Build a top-of-hour UTC timestamp for deterministic benchmarks.
fn dt(year: i16, month: i8, day: i8, hour: i8) -> Timestamp {
    jiff::civil::datetime(year, month, day, hour, 0, 0, 0).in_tz("UTC").unwrap().timestamp()
}

/// Build a minimal event with the given name and start.
fn make_event(name: &str, start: Timestamp) -> Event {
    Event::new(name, start)
}

/// Benchmark title matching for an exact pair and a one-typo fuzzy
/// pair.
fn bench_name_matching(c: &mut Criterion) {
    c.bench_function("name_matching_exact", |b| {
        b.iter(|| {
            black_box(name_matching::match_titles(
                black_box("Annual Conference"),
                black_box("Annual Conference"),
            ))
        });
    });
    c.bench_function("name_matching_fuzzy", |b| {
        b.iter(|| {
            black_box(name_matching::match_titles(
                black_box("Annual Conference"),
                black_box("Annual Conferance"),
            ))
        });
    });
}

/// Benchmark start-date proximity scoring for two close timestamps.
fn bench_time_matching(c: &mut Criterion) {
    let a = dt(2026, 3, 1, 9);
    let b = dt(2026, 3, 1, 10);
    c.bench_function("time_match_close", |c| {
        c.iter(|| black_box(time_matching::match_start_dates(black_box(a), black_box(b))));
    });
}

/// Benchmark `Place ↔ Place` location matching on a cloned venue.
fn bench_location_matching(c: &mut Criterion) {
    let p1 = Place {
        id: None,
        name: "Greek Theatre".into(),
        address: Some(Address {
            use_type: None,
            line1: Some("2700 Hearst Ave".into()),
            line2: None,
            city: Some("Berkeley".into()),
            state: Some("CA".into()),
            postal_code: Some("94720".into()),
            country: Some("US".into()),
        }),
        latitude: Some(37.873),
        longitude: Some(-122.254),
        url: None,
    };
    let p2 = p1.clone();
    let l1 = Location::Place(p1);
    let l2 = Location::Place(p2);
    c.bench_function("location_match_place", |c| {
        c.iter(|| black_box(location_matching::match_location(black_box(&l1), black_box(&l2))));
    });
}

/// Benchmark fuzzy organizer-name matching for two similar parties.
fn bench_party_matching(c: &mut Criterion) {
    let a = Party {
        kind: PartyKind::Organization,
        id: None,
        name: "Cal Performances".into(),
        email: None,
        url: None,
    };
    let b = Party {
        kind: PartyKind::Organization,
        id: None,
        name: "Cal Performances Inc".into(),
        email: None,
        url: None,
    };
    c.bench_function("party_match_fuzzy", |c| {
        c.iter(|| black_box(party_matching::match_party(black_box(&a), black_box(&b))));
    });
}

/// Benchmark identifier matching where the two values differ only in
/// formatting (dashes / spaces / case).
fn bench_identifier_matching(c: &mut Criterion) {
    let a = Identifier::new(IdentifierType::BookingNumber, "sys".into(), "ABC-1234".into());
    let b = Identifier::new(IdentifierType::BookingNumber, "sys".into(), "abc 1234".into());
    c.bench_function("identifier_match_formatting_diff", |c| {
        c.iter(|| black_box(identifier_matching::match_identifier(black_box(&a), black_box(&b))));
    });
}

/// Benchmark the end-to-end probabilistic match of one query against
/// 50 candidate events.
fn bench_probabilistic_match(c: &mut Criterion) {
    let config = MatchingConfig {
        threshold_score: 0.7,
        exact_match_score: 1.0,
        fuzzy_match_score: 0.8,
    };
    let matcher = ProbabilisticMatcher::new(config);
    let query = make_event("Concert", dt(2026, 3, 1, 9));
    let candidates: Vec<Event> = (0..50)
        .map(|i| make_event(&format!("Concert {i}"), dt(2026, 3, 1, 9 + (i as i8 % 8))))
        .collect();
    c.bench_function("probabilistic_match_50_candidates", |c| {
        c.iter(|| black_box(matcher.find_matches(black_box(&query), black_box(&candidates)).unwrap()));
    });
}

/// Benchmark Soundex-based phonetic similarity for two like-sounding
/// names.
fn bench_phonetic(c: &mut Criterion) {
    c.bench_function("phonetic_similarity", |b| {
        b.iter(|| {
            black_box(phonetic::phonetic_similarity(
                black_box("Robert"),
                black_box("Rupert"),
            ))
        });
    });
}

criterion_group!(
    benches,
    bench_name_matching,
    bench_time_matching,
    bench_location_matching,
    bench_party_matching,
    bench_identifier_matching,
    bench_probabilistic_match,
    bench_phonetic,
);
criterion_main!(benches);
