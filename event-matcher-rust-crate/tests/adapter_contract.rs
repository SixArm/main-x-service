#![warn(clippy::pedantic)]

//! Adapter contract test for the `event-matcher` public API.
//!
//! Pins the public surface that downstream `event-service` depends on via
//! its `to_matcher_event` adapter.

use event_matcher::{
    Address, Confidence, Event, EventAttendanceMode, EventBuilder, EventCategory, EventId,
    EventIdScheme, EventStatus, Location, MatchConfig, MatchingEngine,
};

// =============================================================================
// 1. EventBuilder surface
// =============================================================================

#[test]
fn event_builder_full_surface() {
    let loc = Location::new()
        .with_venue_name("Greek Theatre")
        .with_latitude(37.873);
    let eid = EventId::new(EventIdScheme::Eventbrite, "123456789").unwrap();

    let e: Event = Event::builder()
        .name("Annual Conference")
        .add_alternate_name("AnnConf")
        .alternate_names(vec!["The Conf".into()])
        .description("Annual conference")
        .url("https://example.com")
        .event_ids(vec![eid.clone()])
        .add_event_id(eid)
        .local_id("LOCAL-1")
        .category(EventCategory::ConferenceEvent)
        .keywords(vec!["tech".into()])
        .add_keyword("conference")
        .in_language("en")
        .typical_age_range("18+")
        .start_date("2026-06-01T09:00:00Z")
        .end_date("2026-06-01T17:00:00Z")
        .door_time("2026-06-01T08:00:00Z")
        .previous_start_date("2026-05-01T09:00:00Z")
        .event_status(EventStatus::EventScheduled)
        .event_attendance_mode(EventAttendanceMode::OfflineEventAttendanceMode)
        .location(loc)
        .country_code_as_iso_3166_1_alpha_2("US")
        .organizer("Cal Performances")
        .performers(vec!["Alice".into()])
        .add_performer("Bob")
        .maximum_attendee_capacity(1000)
        .maximum_physical_attendee_capacity(800)
        .maximum_virtual_attendee_capacity(200)
        .is_accessible_for_free(true)
        .super_event_id("parent-event-id")
        .build();

    assert_eq!(e.name.as_deref(), Some("Annual Conference"));
    assert_eq!(e.category, Some(EventCategory::ConferenceEvent));
    assert_eq!(e.event_status, Some(EventStatus::EventScheduled));
    assert!(e.location.is_some());
    assert_eq!(e.performers, vec!["Alice".to_string(), "Bob".to_string()]);
}

// =============================================================================
// 2. EventId / EventIdScheme variants the service adapter routes into
// =============================================================================

#[test]
fn event_id_scheme_variants_exist() {
    let _ = [
        EventIdScheme::Wikidata,
        EventIdScheme::Eventbrite,
        EventIdScheme::Meetup,
        EventIdScheme::Ticketmaster,
        EventIdScheme::Songkick,
        EventIdScheme::Bandsintown,
        EventIdScheme::Facebook,
        EventIdScheme::Luma,
        EventIdScheme::GoogleCalendar,
        EventIdScheme::ICalendarUid,
        EventIdScheme::Other("Custom".into()),
    ];
    assert!(EventId::new(EventIdScheme::Eventbrite, "   ").is_none());
    let id = EventId::new(EventIdScheme::Eventbrite, "123").unwrap();
    assert_eq!(id.scheme, EventIdScheme::Eventbrite);
}

// =============================================================================
// 3. EventCategory + EventStatus + EventAttendanceMode variants
// =============================================================================

#[test]
fn event_category_variants_used_by_adapter_exist() {
    let _ = [
        EventCategory::BusinessEvent,
        EventCategory::ChildrensEvent,
        EventCategory::ComedyEvent,
        EventCategory::ConferenceEvent,
        EventCategory::CourseInstance,
        EventCategory::DanceEvent,
        EventCategory::DeliveryEvent,
        EventCategory::EducationEvent,
        EventCategory::EventSeries,
        EventCategory::ExhibitionEvent,
        EventCategory::Festival,
        EventCategory::FoodEvent,
        EventCategory::Hackathon,
        EventCategory::LiteraryEvent,
        EventCategory::MusicEvent,
        EventCategory::PerformingArtsEvent,
        EventCategory::PublicationEvent,
        EventCategory::SaleEvent,
        EventCategory::ScreeningEvent,
        EventCategory::SocialEvent,
        EventCategory::SportsEvent,
        EventCategory::TheaterEvent,
        EventCategory::VisualArtsEvent,
        EventCategory::Other("custom".into()),
    ];
}

#[test]
fn event_status_variants_exist() {
    let _ = [
        EventStatus::EventScheduled,
        EventStatus::EventCancelled,
        EventStatus::EventPostponed,
        EventStatus::EventRescheduled,
        EventStatus::EventMovedOnline,
    ];
}

#[test]
fn event_attendance_mode_variants_exist() {
    let _ = [
        EventAttendanceMode::OfflineEventAttendanceMode,
        EventAttendanceMode::OnlineEventAttendanceMode,
        EventAttendanceMode::MixedEventAttendanceMode,
    ];
}

// =============================================================================
// 4. Location + Address builder surface
// =============================================================================

#[test]
fn location_builder_surface() {
    let addr = Address::new().with_line1("1 Test St").with_city("Town");
    let l = Location::new()
        .with_venue_name("Venue")
        .with_address(addr)
        .with_latitude(40.0)
        .with_longitude(-73.0)
        .with_virtual_url("https://zoom.example/123");
    assert_eq!(l.venue_name.as_deref(), Some("Venue"));
    assert_eq!(l.latitude, Some(40.0));
    assert_eq!(l.longitude, Some(-73.0));
    assert_eq!(l.virtual_url.as_deref(), Some("https://zoom.example/123"));
}

// =============================================================================
// 5. MatchingEngine entry points
// =============================================================================

#[test]
fn matching_engine_constructor_surface() {
    let _: MatchingEngine = MatchingEngine::default_config();
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::default());
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::strict());
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::lenient());
}

#[test]
fn matching_engine_match_events_returns_match_result() {
    let a = Event::builder()
        .name("X")
        .start_date("2026-06-01T09:00:00Z")
        .build();
    let b = a.clone();
    let result = MatchingEngine::default_config().match_events(&a, &b);
    let _: f64 = result.score;
    let _: bool = result.is_match;
    let _: Confidence = result.confidence;
    let _ = result.breakdown.name_score;
    let _ = result.breakdown.start_date_score;
    let _ = result.breakdown.end_date_score;
    let _ = result.breakdown.location_score;
    let _ = result.breakdown.category_score;
    let _ = result.breakdown.country_code_score;
    let _ = result.breakdown.event_ids_score;
    let _ = result.breakdown.organizer_score;
    let _ = result.breakdown.performers_score;
    let _ = result.breakdown.url_score;
}

#[test]
fn matching_engine_deterministic_match_returns_bool() {
    let id = EventId::new(EventIdScheme::Eventbrite, "999").unwrap();
    let a = Event::builder()
        .name("A")
        .start_date("2026-06-01T09:00:00Z")
        .add_event_id(id.clone())
        .build();
    let b = Event::builder()
        .name("B")
        .start_date("2027-06-01T09:00:00Z")
        .add_event_id(id)
        .build();
    let res: bool = MatchingEngine::default_config().deterministic_match(&a, &b);
    assert!(res, "shared event_id must trigger deterministic match");
}

#[test]
fn matching_engine_match_one_to_many_returns_vec() {
    let query = Event::builder()
        .name("Q")
        .start_date("2026-06-01T09:00:00Z")
        .build();
    let candidates = vec![query.clone(), query.clone()];
    let results = MatchingEngine::default_config().match_one_to_many(&query, &candidates);
    assert_eq!(results.len(), candidates.len());
}

// =============================================================================
// 6. Confidence + config + round-trip
// =============================================================================

#[test]
fn confidence_variants_exist() {
    let _ = [Confidence::High, Confidence::Medium, Confidence::Low];
}

#[test]
fn match_config_preset_scores_form_monotonic_threshold_ladder() {
    let strict = MatchConfig::strict().match_threshold;
    let default = MatchConfig::default().match_threshold;
    let lenient = MatchConfig::lenient().match_threshold;
    assert!(strict >= default && default >= lenient);
}

#[test]
fn match_result_round_trips_through_json() {
    let a = Event::builder()
        .name("X")
        .start_date("2026-06-01T09:00:00Z")
        .build();
    let result = MatchingEngine::default_config().match_events(&a, &a);
    let json = serde_json::to_string(&result).expect("serialize");
    let _: event_matcher::MatchResult = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn event_builder_is_value_type() {
    fn _check(b: EventBuilder) -> EventBuilder {
        b.name("ok")
    }
}
