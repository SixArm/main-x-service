//! Benchmarks for event validation and normalization.

use chrono::{TimeZone, Utc};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use event_service::models::*;
use event_service::validation::{normalize_phone, standardize_address, validate_event};

fn make_event(name: &str) -> Event {
    Event::new(name, Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).unwrap())
}

fn bench_validate_simple_event(c: &mut Criterion) {
    let event = make_event("Annual Conference");
    c.bench_function("validate_simple_event", |b| {
        b.iter(|| black_box(validate_event(black_box(&event))));
    });
}

fn bench_validate_rich_event(c: &mut Criterion) {
    let mut event = make_event("Concert");
    event.event_type = EventType::Music;
    event.event_attendance_mode = EventAttendanceMode::Mixed;
    event.location.push(Location::Place(Place {
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
    }));
    event.location.push(Location::Virtual(VirtualLocation {
        name: Some("Livestream".into()),
        url: "https://example.test/stream".into(),
    }));
    event.organizers.push(Party {
        kind: PartyKind::Organization,
        id: None,
        name: "Cal Performances".into(),
        email: Some("info@example.test".into()),
        url: None,
    });
    event.keywords.extend(["music".to_string(), "live".to_string()]);
    event.in_language.push("en".into());

    c.bench_function("validate_rich_event", |b| {
        b.iter(|| black_box(validate_event(black_box(&event))));
    });
}

fn bench_normalize_phone(c: &mut Criterion) {
    c.bench_function("normalize_phone_us", |b| {
        b.iter(|| black_box(normalize_phone(black_box("(555) 123-4567"), black_box("1"))));
    });
}

fn bench_standardize_address(c: &mut Criterion) {
    let addr = Address {
        use_type: None,
        line1: Some("100 Oak Ave.".into()),
        line2: None,
        city: Some("los angeles".into()),
        state: Some("ca".into()),
        postal_code: Some("90001".into()),
        country: Some("us".into()),
    };
    c.bench_function("standardize_address", |b| {
        b.iter(|| black_box(standardize_address(black_box(&addr))));
    });
}

criterion_group!(
    benches,
    bench_validate_simple_event,
    bench_validate_rich_event,
    bench_normalize_phone,
    bench_standardize_address,
);
criterion_main!(benches);
