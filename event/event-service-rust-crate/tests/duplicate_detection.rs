#![warn(clippy::pedantic)]

//! End-to-end duplicate-detection integration tests for the
//! event-service ↔ event-matcher bridge.
//!
//! Exercises the schema.org/Event identity model: DateTime → ISO 8601
//! string projection, `Vec<Location>` → first matcher `Location` dispatch
//! (Place / PostalAddress / Virtual / Text), `Vec<Party>` → single matcher
//! organizer / performers list, identifier scheme mapping by system URI
//! (Eventbrite, Meetup, Wikidata, Google Calendar, iCalendar).

use chrono::{DateTime, TimeZone, Utc};

use event_service::matching::adapter::to_matcher_event;
use event_service::matching::matcher_lib::{Confidence, MatchingEngine};
use event_service::models::{
    Address, AddressUse, Event, EventStatus, EventType, Location, Party, PartyKind, Place,
    VirtualLocation,
    identifier::{Identifier, IdentifierType},
};

// -------- builders -----------------------------------------------------------

/// Build a fixed top-of-hour UTC timestamp for deterministic tests.
fn start_at(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, 0, 0).unwrap()
}

/// Build a scheduled conference-type event with the given name/start.
fn event(name: &str, start: DateTime<Utc>) -> Event {
    let mut e = Event::new(name, start);
    e.event_type = EventType::Conference;
    e.event_status = EventStatus::Scheduled;
    e
}

/// A fully-populated reference conference (location + organizer) used
/// as a fixture across the bridge tests.
fn conference() -> Event {
    let mut e = event("Annual Conference", start_at(2026, 6, 1, 9));
    e.end_date = Some(start_at(2026, 6, 1, 17));
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
    e
}

/// A matching engine with the default configuration.
fn engine() -> MatchingEngine {
    MatchingEngine::default_config()
}

// =============================================================================
// Identical / near-duplicate cases
// =============================================================================

/// An exact clone scores ≥ 0.95 with `High` confidence.
#[test]
fn identical_clones_score_near_one_high_confidence() {
    let a = conference();
    let b = a.clone();

    let result = engine().match_events(&to_matcher_event(&a), &to_matcher_event(&b));

    assert!(
        result.score >= 0.95,
        "identical clones should score ≥ 0.95, got {}",
        result.score
    );
    assert_eq!(result.confidence, Confidence::High);
    assert!(result.is_match);
}

/// A name typo still matches when time and location agree.
#[test]
fn typo_in_name_still_matches_when_time_and_location_agree() {
    let a = conference();
    let mut b = conference();
    b.name = "Annual Conferance".into(); // typo

    let result = engine().match_events(&to_matcher_event(&a), &to_matcher_event(&b));
    assert!(
        result.score >= 0.85,
        "name typo with matching time+location should score ≥ 0.85, got {}",
        result.score
    );
    assert!(result.is_match);
}

/// Closer start dates outscore farther ones for the same name.
#[test]
fn closer_start_dates_outscore_farther_start_dates_for_same_name() {
    // Same name + same location pair, only `start_date` varies. The matcher
    // uses Gaussian-decay over the start-time delta, so closer → higher
    // score. Assert the *ranking*, not an absolute threshold.
    let start = start_at(2026, 6, 1, 9);
    let mut close_a = conference();
    let mut close_b = conference();
    close_a.start_date = start;
    close_b.start_date = start + chrono::Duration::minutes(15); // 15 min drift

    let mut far_a = conference();
    let mut far_b = conference();
    far_a.start_date = start;
    far_b.start_date = start + chrono::Duration::hours(24 * (30)); // 30 days off

    let close = engine().match_events(&to_matcher_event(&close_a), &to_matcher_event(&close_b));
    let far = engine().match_events(&to_matcher_event(&far_a), &to_matcher_event(&far_b));

    assert!(
        close.score > far.score,
        "close start_date (15 min) must outscore far (30 days): close={} far={}",
        close.score,
        far.score
    );
}

// =============================================================================
// Deterministic short-circuits — strong identifiers
// =============================================================================

/// A shared Eventbrite id (routed by system URI) triggers a
/// deterministic match.
#[test]
fn shared_eventbrite_identifier_via_system_uri_is_deterministic() {
    let mut a = event("Tech Talk", start_at(2026, 7, 15, 18));
    a.identifiers.push(Identifier {
        use_type: None,
        identifier_type: IdentifierType::ExternalRef,
        system: "https://eventbrite.com/event-id".into(),
        value: "123456789".into(),
        assigner: None,
    });
    let mut b = event("Tech Talk (Updated)", start_at(2026, 7, 15, 18));
    b.identifiers.push(Identifier {
        use_type: None,
        identifier_type: IdentifierType::ExternalRef,
        system: "https://eventbrite.com/event-id".into(),
        value: "123456789".into(),
        assigner: None,
    });

    let ma = to_matcher_event(&a);
    let mb = to_matcher_event(&b);

    assert!(
        engine().deterministic_match(&ma, &mb),
        "shared Eventbrite ID must trigger deterministic_match"
    );
    let result = engine().match_events(&ma, &mb);
    assert_eq!(result.breakdown.event_ids_score, Some(1.0));
}

/// A shared iCalendar UID (routed by system URI) triggers a
/// deterministic match.
#[test]
fn icalendar_uid_routing_via_system_uri() {
    let mut a = event("Standup", start_at(2026, 7, 16, 9));
    a.identifiers.push(Identifier {
        use_type: None,
        identifier_type: IdentifierType::Other,
        system: "urn:ietf:rfc:5545:ical".into(),
        value: "20260716T090000Z-standup@example.com".into(),
        assigner: None,
    });
    let mut b = event("Standup", start_at(2026, 7, 16, 9));
    b.identifiers.push(Identifier {
        use_type: None,
        identifier_type: IdentifierType::Other,
        system: "urn:ietf:rfc:5545:ical".into(),
        value: "20260716T090000Z-standup@example.com".into(),
        assigner: None,
    });

    assert!(
        engine().deterministic_match(&to_matcher_event(&a), &to_matcher_event(&b)),
        "shared iCalendar UID must trigger deterministic_match"
    );
}

/// With no system hint, a `BookingNumber` falls back to the type enum
/// and still produces an identifier-axis match.
#[test]
fn booking_number_via_type_fallback_when_no_system_hint() {
    // No system URI → adapter falls back to the IdentifierType enum and
    // emits an Other("BookingNumber") scheme.
    let mut a = event("Dinner", start_at(2026, 8, 1, 19));
    a.identifiers.push(Identifier {
        use_type: None,
        identifier_type: IdentifierType::BookingNumber,
        system: "https://reservations.example.com".into(),
        value: "ABCD-1234".into(),
        assigner: None,
    });
    let mut b = event("Dinner reservation", start_at(2026, 8, 1, 19));
    b.identifiers.push(Identifier {
        use_type: None,
        identifier_type: IdentifierType::BookingNumber,
        system: "https://reservations.example.com".into(),
        value: "ABCD-1234".into(),
        assigner: None,
    });

    let ma = to_matcher_event(&a);
    let mb = to_matcher_event(&b);
    assert_eq!(
        ma.event_ids.len(),
        1,
        "BookingNumber should land as one EventId"
    );
    // Same (scheme, value) on each side → identifier-axis equality.
    let result = engine().match_events(&ma, &mb);
    assert_eq!(result.breakdown.event_ids_score, Some(1.0));
}

// =============================================================================
// Location dispatch — Place vs Virtual vs PostalAddress
// =============================================================================

/// A shared virtual-meeting URL drives a positive location score.
#[test]
fn virtual_location_url_match_drives_location_score() {
    let mut a = event("Webinar", start_at(2026, 6, 1, 10));
    a.location = vec![Location::Virtual(VirtualLocation {
        name: Some("Zoom".into()),
        url: "https://example.zoom.us/j/123".into(),
    })];
    let mut b = event("Webinar", start_at(2026, 6, 1, 10));
    b.location = vec![Location::Virtual(VirtualLocation {
        name: Some("Zoom".into()),
        url: "https://example.zoom.us/j/123".into(),
    })];

    let result = engine().match_events(&to_matcher_event(&a), &to_matcher_event(&b));
    assert!(
        result.breakdown.location_score.unwrap_or(0.0) >= 0.5,
        "shared virtual-meeting URL must drive a positive location_score, got {:?}",
        result.breakdown.location_score
    );
}

/// A `Place` location's name and geo coords propagate into the matcher
/// `Location`.
#[test]
fn place_location_geo_propagates_into_matcher_location() {
    let mut a = event("Gig", start_at(2026, 6, 1, 20));
    a.location = vec![Location::Place(Place {
        id: None,
        name: "Greek Theatre".into(),
        address: None,
        latitude: Some(37.8730),
        longitude: Some(-122.2547),
        url: None,
    })];

    let m = to_matcher_event(&a);
    let loc = m.location.as_ref().unwrap();
    assert_eq!(loc.venue_name.as_deref(), Some("Greek Theatre"));
    assert_eq!(loc.latitude, Some(37.8730));
}

// =============================================================================
// Organizer / party dispatch
// =============================================================================

/// Only the first organizer name populates the matcher's single
/// organizer slot.
#[test]
fn first_organizer_name_propagates_to_matcher_organizer_slot() {
    let mut a = event("Show", start_at(2026, 8, 1, 20));
    a.organizers = vec![
        Party {
            kind: PartyKind::Organization,
            id: None,
            name: "Cal Performances".into(),
            email: None,
            url: None,
        },
        Party {
            kind: PartyKind::Organization,
            id: None,
            name: "Co-sponsor".into(),
            email: None,
            url: None,
        },
    ];

    let m = to_matcher_event(&a);
    assert_eq!(
        m.organizer.as_deref(),
        Some("Cal Performances"),
        "first organizer name must populate matcher.organizer (single string slot)"
    );
}

/// Every performer name passes through to the matcher's `performers`
/// string vector, in order.
#[test]
fn performers_pass_through_as_string_vec() {
    let mut a = event("Concert", start_at(2026, 9, 1, 20));
    a.performers = vec![
        Party {
            kind: PartyKind::Person,
            id: None,
            name: "Alice".into(),
            email: None,
            url: None,
        },
        Party {
            kind: PartyKind::Person,
            id: None,
            name: "Bob".into(),
            email: None,
            url: None,
        },
    ];
    let m = to_matcher_event(&a);
    assert_eq!(m.performers, vec!["Alice".to_string(), "Bob".to_string()]);
}

// =============================================================================
// Negative cases
// =============================================================================

/// Unrelated events (different name, time, place, type) score below
/// 0.50 and are not flagged as a match.
#[test]
fn completely_different_events_score_low_and_do_not_match() {
    let a = conference();
    let mut b = event("Beach Party", start_at(2030, 1, 1, 12));
    b.location = vec![Location::Place(Place {
        id: None,
        name: "Bondi Beach".into(),
        address: None,
        latitude: Some(-33.89),
        longitude: Some(151.27),
        url: None,
    })];
    b.event_type = EventType::Social;

    let result = engine().match_events(&to_matcher_event(&a), &to_matcher_event(&b));
    assert!(
        result.score < 0.50,
        "unrelated events should score < 0.50, got {}",
        result.score
    );
    assert!(!result.is_match);
}

/// An identical name six years apart must not reach the High band —
/// time divergence prevents a deterministic short-circuit.
#[test]
fn same_name_far_apart_dates_does_not_short_circuit() {
    let mut a = event("Annual Gala", start_at(2024, 6, 1, 19));
    a.location = vec![]; // remove location to force time to dominate
    let mut b = event("Annual Gala", start_at(2030, 6, 1, 19));
    b.location = vec![];

    let result = engine().match_events(&to_matcher_event(&a), &to_matcher_event(&b));
    assert!(
        result.score < 0.90,
        "same name 6-years apart must not hit High band, got {}",
        result.score
    );
}

// =============================================================================
// Field-routing pinning
// =============================================================================

/// `EventType::Conference` maps to the matcher's
/// `EventCategory::ConferenceEvent`.
#[test]
fn event_type_conference_maps_to_matcher_conference_event() {
    use event_service::matching::matcher_lib::EventCategory;
    let e = event("Talk", start_at(2026, 6, 1, 10));
    let m = to_matcher_event(&e);
    assert_eq!(m.category, Some(EventCategory::ConferenceEvent));
}

/// `EventStatus::Scheduled` maps to the matcher's
/// `EventStatus::EventScheduled`.
#[test]
fn event_status_scheduled_maps_to_matcher_event_scheduled() {
    use event_service::matching::matcher_lib::EventStatus as MEventStatus;
    let e = event("Talk", start_at(2026, 6, 1, 10));
    let m = to_matcher_event(&e);
    assert_eq!(m.event_status, Some(MEventStatus::EventScheduled));
}

/// The domain `Timestamp` start date projects to an RFC 3339
/// string on the matcher side.
#[test]
fn start_date_serialises_to_rfc3339() {
    let e = event("Talk", start_at(2026, 6, 1, 9));
    let m = to_matcher_event(&e);
    assert!(
        m.start_date
            .as_deref()
            .unwrap()
            .starts_with("2026-06-01T09:00:00"),
        "start_date must be RFC 3339 with UTC offset, got {:?}",
        m.start_date
    );
}

// =============================================================================
// Edge cases
// =============================================================================

/// Near-empty records score without panicking and stay within
/// `[0.0, 1.0]`.
#[test]
fn sparse_records_do_not_panic_and_stay_in_range() {
    let a = event("", start_at(2026, 1, 1, 0));
    let b = event("X", start_at(2026, 1, 1, 0));
    let result = engine().match_events(&to_matcher_event(&a), &to_matcher_event(&b));
    assert!(result.score >= 0.0 && result.score <= 1.0);
}

// =============================================================================
// Language tag (`in_language`) — ET-8 contract pinning
// =============================================================================
//
// The service validates `in_language` as 2-letter ISO 639-1 codes; the
// embedded `event-matcher` documents the field as a full IETF BCP 47 tag
// (ISO 639-1 ⊂ BCP 47). The bridge (`src/matching/adapter.rs`) projects the
// FIRST non-empty `in_language` entry onto the matcher's single
// `Option<String> in_language` field, but the matcher treats that field as
// data-only: there is no `in_language_score` in `MatchBreakdown` and the
// probabilistic pipeline never reads it. The divergence is therefore inert.
//
// These two tests lock the documented contract (service spec §6.2,
// `AGENTS/matching.md`, entity task ET-8) so a future change to either side
// — scoring the field, or dropping the projection — fails here loudly.

/// The adapter projects the FIRST `in_language` entry onto the matcher's
/// single `in_language` field; later entries are dropped (the matcher
/// carries only one tag). A leading empty/whitespace entry projects
/// nothing — the adapter inspects only `.first()`, it does not scan ahead
/// for the first *non-empty* entry.
#[test]
fn in_language_first_entry_is_projected() {
    // First entry populated: it is carried across, later entries dropped.
    let mut e = event("Polyglot Summit", start_at(2026, 6, 1, 9));
    e.in_language = vec!["en".into(), "fr".into()];
    assert_eq!(
        to_matcher_event(&e).in_language.as_deref(),
        Some("en"),
        "adapter must project the first in_language entry"
    );

    // First entry empty: nothing is projected (no scan-ahead).
    let mut blank_first = event("Polyglot Summit", start_at(2026, 6, 1, 9));
    blank_first.in_language = vec![String::new(), "fr".into()];
    assert_eq!(
        to_matcher_event(&blank_first).in_language,
        None,
        "a leading empty entry projects nothing (adapter reads only .first())"
    );
}

/// Two events that are identical apart from `in_language` still match with
/// the same score: the matcher never scores the field, so it is inert.
#[test]
fn in_language_difference_does_not_affect_match() {
    // Baseline: a fully-populated conference compared against its clone.
    let base = conference();
    let baseline = engine().match_events(&to_matcher_event(&base), &to_matcher_event(&base));

    // Same pair, but the two sides now carry different language tags.
    let mut a = conference();
    let mut b = conference();
    a.in_language = vec!["en".into()];
    b.in_language = vec!["fr".into()];
    let differing = engine().match_events(&to_matcher_event(&a), &to_matcher_event(&b));

    // The projection happened (field is present on both sides) ...
    assert_eq!(to_matcher_event(&a).in_language.as_deref(), Some("en"));
    assert_eq!(to_matcher_event(&b).in_language.as_deref(), Some("fr"));

    // ... yet the differing language tags leave the score untouched.
    assert!(
        (differing.score - baseline.score).abs() < f64::EPSILON,
        "in_language is inert: differing tags changed the score \
         (baseline {}, differing {})",
        baseline.score,
        differing.score
    );
    assert!(differing.is_match, "the events should still be a match");
}
