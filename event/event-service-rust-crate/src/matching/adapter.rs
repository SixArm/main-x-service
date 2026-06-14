//! Adapter from the service's `Event` domain model to the `event-matcher`
//! library's `Event` matching input.
//!
//! The service's `Event` is schema.org/Event-aligned and uses typed
//! `Timestamp` for all time fields, `Vec<Location>` (a tagged union of
//! `Place` / `PostalAddress` / `Virtual` / `Text`), `Vec<Party>` for
//! organizers/performers/attendees, and `Vec<Identifier>` with an enumerated
//! `IdentifierType`. The matcher's `Event` accepts ISO 8601 *strings* for
//! every time field, a single `Option<Location>`, a single
//! `Option<String> organizer`, and a `Vec<EventId>` keyed by
//! `EventIdScheme`.
//!
//! [`to_matcher_event`](crate::matching::adapter::to_matcher_event)
//! performs the projection — including:
//!
//! - DateTime → RFC 3339 string
//! - First `Location` → matcher's `Location` struct (variant-aware)
//! - First organizer name → `organizer`
//! - Each performer's name → `performers`
//! - Each `Identifier` → `EventId` (via
//!   [`map_identifier_scheme`](crate::matching::adapter::map_identifier_scheme))
//! - `EventStatus` / `EventAttendanceMode` / `EventType` → matcher enums
//!
//! See `agents/share/match.md` and the matcher crate's `spec.md §5–§7` for
//! the algorithm contract this adapter feeds.

use event_matcher::{
    Address as MAddress, Event as MEvent, EventAttendanceMode as MMode, EventCategory as MCategory,
    EventId as MEventId, EventIdScheme as MScheme, EventStatus as MStatus, Location as MLocation,
};

use crate::models::{
    Address as SAddress, Event, EventAttendanceMode, EventStatus, EventType, Location, Party,
    identifier::{Identifier, IdentifierType},
};

/// Convert a service [`Event`] into an `event_matcher::Event` ready for
/// `MatchingEngine::match_events` / `deterministic_match`.
///
/// Empty / whitespace-only strings are dropped rather than projected,
/// and only the *first* populated location and the *first* non-empty
/// organizer name are carried across (the matcher accepts a single
/// `Location` and a single organizer).
pub fn to_matcher_event(e: &Event) -> MEvent {
    let mut b = MEvent::builder().name(e.name.trim());

    let alts: Vec<String> = e
        .alternate_names
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if !alts.is_empty() {
        b = b.alternate_names(alts);
    }

    if let Some(d) = e
        .description
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        b = b.description(d);
    }
    if let Some(u) = e.url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        b = b.url(u);
    }

    // Identifiers → EventIds (best-effort scheme mapping).
    let ids: Vec<MEventId> = e
        .identifiers
        .iter()
        .filter_map(identifier_to_event_id)
        .collect();
    if !ids.is_empty() {
        b = b.event_ids(ids);
    }

    let keywords: Vec<String> = e
        .keywords
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if !keywords.is_empty() {
        b = b.keywords(keywords);
    }

    if let Some(lang) = e.in_language.first() {
        let lang = lang.trim();
        if !lang.is_empty() {
            b = b.in_language(lang);
        }
    }

    if let Some(age) = e
        .typical_age_range
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        b = b.typical_age_range(age);
    }

    // Times: DateTime<Utc> → RFC 3339.
    b = b.start_date(e.start_date.to_rfc3339());
    if let Some(t) = e.end_date {
        b = b.end_date(t.to_rfc3339());
    }
    if let Some(t) = e.door_time {
        b = b.door_time(t.to_rfc3339());
    }
    if let Some(t) = e.previous_start_date {
        b = b.previous_start_date(t.to_rfc3339());
    }

    b = b.event_status(map_status(e.event_status));
    b = b.event_attendance_mode(map_attendance_mode(e.event_attendance_mode));
    b = b.category(map_event_type(&e.event_type));

    // Location: matcher takes one; service takes Vec — use the first
    // populated location.
    if let Some(loc) = e.location.iter().find_map(map_location) {
        b = b.location(loc);
    }

    // Organizer: matcher takes one name; service has Vec<Party> — take the
    // first non-empty name.
    if let Some(o) = first_party_name(&e.organizers) {
        b = b.organizer(o);
    }
    // Performers: matcher takes Vec<String>; service takes Vec<Party>.
    let perfs: Vec<String> = e
        .performers
        .iter()
        .map(|p| p.name.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if !perfs.is_empty() {
        b = b.performers(perfs);
    }

    if let Some(c) = e.maximum_attendee_capacity {
        b = b.maximum_attendee_capacity(c);
    }
    if let Some(c) = e.maximum_physical_attendee_capacity {
        b = b.maximum_physical_attendee_capacity(c);
    }
    if let Some(c) = e.maximum_virtual_attendee_capacity {
        b = b.maximum_virtual_attendee_capacity(c);
    }
    if let Some(v) = e.is_accessible_for_free {
        b = b.is_accessible_for_free(v);
    }
    if let Some(parent) = e.super_event {
        b = b.super_event_id(parent.to_string());
    }

    b.build()
}

/// Return the first non-empty (trimmed) party name from a `Vec<Party>`.
///
/// The matcher carries a single `organizer` string, so when the service
/// side has several parties we pick the first one with a usable name.
fn first_party_name(parties: &[Party]) -> Option<String> {
    parties
        .iter()
        .map(|p| p.name.trim().to_string())
        .find(|s| !s.is_empty())
}

/// Map the service `EventStatus` to the matcher `EventStatus`.
///
/// The matcher has no `Completed` variant, so a completed event is
/// projected as `EventScheduled` (it ran on schedule).
fn map_status(s: EventStatus) -> MStatus {
    match s {
        EventStatus::Scheduled => MStatus::EventScheduled,
        EventStatus::Cancelled => MStatus::EventCancelled,
        EventStatus::Postponed => MStatus::EventPostponed,
        EventStatus::Rescheduled => MStatus::EventRescheduled,
        EventStatus::MovedOnline => MStatus::EventMovedOnline,
        // The matcher does not carry a Completed variant — closest semantic
        // is Scheduled (the event happened on schedule).
        EventStatus::Completed => MStatus::EventScheduled,
    }
}

/// Map the service `EventAttendanceMode` to the matcher equivalent
/// (`Offline` / `Online` / `Mixed`).
fn map_attendance_mode(m: EventAttendanceMode) -> MMode {
    match m {
        EventAttendanceMode::Offline => MMode::OfflineEventAttendanceMode,
        EventAttendanceMode::Online => MMode::OnlineEventAttendanceMode,
        EventAttendanceMode::Mixed => MMode::MixedEventAttendanceMode,
    }
}

/// Map the service's `EventType` enum to the matcher's `EventCategory`.
///
/// Where the matcher has a direct counterpart (e.g. `Conference` ↔
/// `ConferenceEvent`) we use it; operational subtypes the matcher does not
/// model (`Appointment`, `Encounter`, `Shift`, `Incident`, `Generic`,
/// `Session`, `Course`, `Series`) flow through as `Other(name)` so the
/// scheme name still participates in matching.
pub fn map_event_type(t: &EventType) -> MCategory {
    match t {
        EventType::Business => MCategory::BusinessEvent,
        EventType::Childrens => MCategory::ChildrensEvent,
        EventType::Comedy => MCategory::ComedyEvent,
        EventType::Conference => MCategory::ConferenceEvent,
        EventType::Course => MCategory::CourseInstance,
        EventType::Dance => MCategory::DanceEvent,
        EventType::Delivery => MCategory::DeliveryEvent,
        EventType::Education => MCategory::EducationEvent,
        EventType::Exhibition => MCategory::ExhibitionEvent,
        EventType::Festival => MCategory::Festival,
        EventType::Food => MCategory::FoodEvent,
        EventType::Hackathon => MCategory::Hackathon,
        EventType::Literary => MCategory::LiteraryEvent,
        EventType::Music => MCategory::MusicEvent,
        EventType::PerformingArts => MCategory::PerformingArtsEvent,
        EventType::Publication => MCategory::PublicationEvent,
        EventType::Sale => MCategory::SaleEvent,
        EventType::Screening => MCategory::ScreeningEvent,
        EventType::Series => MCategory::EventSeries,
        EventType::Social => MCategory::SocialEvent,
        EventType::Sports => MCategory::SportsEvent,
        EventType::Theater => MCategory::TheaterEvent,
        EventType::VisualArts => MCategory::VisualArtsEvent,
        EventType::Generic => MCategory::Other("Generic".into()),
        EventType::Appointment => MCategory::Other("Appointment".into()),
        EventType::Encounter => MCategory::Other("Encounter".into()),
        EventType::Shift => MCategory::Other("Shift".into()),
        EventType::Incident => MCategory::Other("Incident".into()),
        EventType::Session => MCategory::Other("Session".into()),
    }
}

/// Project one service [`Location`] variant onto the matcher's single
/// `Location` struct.
///
/// Each variant contributes whatever fields it carries (venue name,
/// address, geo coords, virtual URL). Returns `None` when the location
/// is effectively empty, so the caller can skip to the next candidate.
fn map_location(l: &Location) -> Option<MLocation> {
    let mut m = MLocation::new();
    let mut any = false;
    match l {
        Location::Place(p) => {
            if !p.name.trim().is_empty() {
                m = m.with_venue_name(p.name.trim());
                any = true;
            }
            if let Some(a) = p.address.as_ref().and_then(map_service_address) {
                m = m.with_address(a);
                any = true;
            }
            if let Some(lat) = p.latitude {
                m = m.with_latitude(lat);
                any = true;
            }
            if let Some(lon) = p.longitude {
                m = m.with_longitude(lon);
                any = true;
            }
        }
        Location::PostalAddress(a) => {
            if let Some(addr) = map_service_address(a) {
                m = m.with_address(addr);
                any = true;
            }
        }
        Location::Virtual(v) => {
            let url = v.url.trim();
            if !url.is_empty() {
                m = m.with_virtual_url(url);
                any = true;
            }
            if let Some(n) = v.name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                m = m.with_venue_name(n);
                any = true;
            }
        }
        Location::Text { value } => {
            let v = value.trim();
            if !v.is_empty() {
                m = m.with_venue_name(v);
                any = true;
            }
        }
    }
    if any { Some(m) } else { None }
}

/// Project a service [`Address`](crate::models::Address) onto the
/// matcher's `Address`.
///
/// Returns `None` when every field is absent. Note the field rename:
/// the service's `state` becomes the matcher's British `county`.
fn map_service_address(a: &SAddress) -> Option<MAddress> {
    let any = a.line1.is_some()
        || a.line2.is_some()
        || a.city.is_some()
        || a.state.is_some()
        || a.postal_code.is_some()
        || a.country.is_some();
    if !any {
        return None;
    }
    let mut m = MAddress::new();
    if let Some(v) = a.line1.as_deref() {
        m = m.with_line1(v);
    }
    if let Some(v) = a.line2.as_deref() {
        m = m.with_line2(v);
    }
    if let Some(v) = a.city.as_deref() {
        m = m.with_city(v);
    }
    if let Some(v) = a.state.as_deref() {
        m = m.with_county(v); // matcher uses British "county"; service uses "state"
    }
    if let Some(v) = a.postal_code.as_deref() {
        m = m.with_postcode(v);
    }
    if let Some(v) = a.country.as_deref() {
        m = m.with_country(v);
    }
    Some(m)
}

/// Convert one service [`Identifier`] into a matcher `EventId`, deriving
/// the scheme from its type + system URI via [`map_identifier_scheme`].
///
/// Returns `None` when the value is empty (matcher `EventId::new`
/// rejects blank values).
fn identifier_to_event_id(id: &Identifier) -> Option<MEventId> {
    MEventId::new(
        map_identifier_scheme(&id.identifier_type, &id.system),
        id.value.trim(),
    )
}

/// Map the service's `IdentifierType` + `system` URI to a matcher
/// `EventIdScheme`.
///
/// The matcher's vocabulary is venue/ticketing-centric (Eventbrite,
/// Meetup, Ticketmaster, Songkick, Wikidata, …); the service's vocabulary
/// is operational (BookingNumber, ConfirmationCode, TicketNumber, …). When
/// the `system` URI hints at one of the matcher's known schemes that wins,
/// otherwise the identifier-type name flows through as `Other(name)` so the
/// scheme label still participates in `(scheme, value)` equality.
pub fn map_identifier_scheme(t: &IdentifierType, system: &str) -> MScheme {
    let sys = system.to_ascii_lowercase();
    if sys.contains("eventbrite") {
        return MScheme::Eventbrite;
    }
    if sys.contains("meetup") {
        return MScheme::Meetup;
    }
    if sys.contains("ticketmaster") {
        return MScheme::Ticketmaster;
    }
    if sys.contains("songkick") {
        return MScheme::Songkick;
    }
    if sys.contains("bandsintown") {
        return MScheme::Bandsintown;
    }
    if sys.contains("facebook") || sys.contains("fb.com") {
        return MScheme::Facebook;
    }
    if sys.contains("lu.ma") || sys.contains("luma") {
        return MScheme::Luma;
    }
    if sys.contains("google") || sys.contains("calendar.google") {
        return MScheme::GoogleCalendar;
    }
    if sys.contains("wikidata") {
        return MScheme::Wikidata;
    }
    if sys.contains("ical") || sys.contains("vcal") {
        return MScheme::ICalendarUid;
    }
    MScheme::Other(match t {
        IdentifierType::BookingNumber => "BookingNumber".into(),
        IdentifierType::ConfirmationCode => "ConfirmationCode".into(),
        IdentifierType::TicketNumber => "TicketNumber".into(),
        IdentifierType::EncounterId => "EncounterId".into(),
        IdentifierType::TransactionId => "TransactionId".into(),
        IdentifierType::ExternalRef => "ExternalRef".into(),
        IdentifierType::Tax => "Tax".into(),
        IdentifierType::Other => "Other".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        Event, EventStatus, Location, Party, PartyKind, Place, VirtualLocation,
        identifier::Identifier,
    };
    use chrono::TimeZone;

    /// Build a minimal service `Event` with a fixed start date for tests.
    fn svc_event(name: &str) -> Event {
        Event::new(
            name,
            chrono::Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap(),
        )
    }

    /// Name, category, status, and start date survive the projection.
    #[test]
    fn round_trip_basic() {
        let mut e = svc_event("Annual Conference");
        e.event_type = EventType::Conference;
        e.event_status = EventStatus::Scheduled;
        let m = to_matcher_event(&e);
        assert_eq!(m.name.as_deref(), Some("Annual Conference"));
        assert_eq!(m.category, Some(MCategory::ConferenceEvent));
        assert_eq!(m.event_status, Some(MStatus::EventScheduled));
        assert!(m.start_date.is_some());
    }

    /// A `Place` location carries its venue name and latitude across.
    #[test]
    fn place_location_maps_to_venue_name_and_geo() {
        let mut e = svc_event("Gig");
        e.location = vec![Location::Place(Place {
            id: None,
            name: "Greek Theatre".into(),
            address: None,
            latitude: Some(37.8730),
            longitude: Some(-122.2547),
            url: None,
        })];
        let m = to_matcher_event(&e);
        let loc = m.location.as_ref().unwrap();
        assert_eq!(loc.venue_name.as_deref(), Some("Greek Theatre"));
        assert_eq!(loc.latitude, Some(37.8730));
    }

    /// A `Virtual` location carries its URL into `virtual_url`.
    #[test]
    fn virtual_location_maps_to_virtual_url() {
        let mut e = svc_event("Webinar");
        e.location = vec![Location::Virtual(VirtualLocation {
            name: Some("Zoom".into()),
            url: "https://example.zoom.us/j/123".into(),
        })];
        let m = to_matcher_event(&e);
        assert_eq!(
            m.location.as_ref().unwrap().virtual_url.as_deref(),
            Some("https://example.zoom.us/j/123")
        );
    }

    /// The first organizer's name becomes the matcher's single organizer.
    #[test]
    fn organizer_takes_first_party_name() {
        let mut e = svc_event("Event");
        e.organizers = vec![Party {
            kind: PartyKind::Organization,
            id: None,
            name: "Cal Performances".into(),
            email: None,
            url: None,
        }];
        let m = to_matcher_event(&e);
        assert_eq!(m.organizer.as_deref(), Some("Cal Performances"));
    }

    /// An Eventbrite `system` URI maps to `EventIdScheme::Eventbrite`
    /// even when the identifier type is the generic `ExternalRef`.
    #[test]
    fn identifier_eventbrite_recognised_via_system_uri() {
        let mut e = svc_event("Event");
        e.identifiers.push(Identifier {
            use_type: None,
            identifier_type: IdentifierType::ExternalRef,
            system: "https://eventbrite.com/event-id".into(),
            value: "123456789".into(),
            assigner: None,
        });
        let m = to_matcher_event(&e);
        assert_eq!(m.event_ids.len(), 1);
        assert_eq!(m.event_ids[0].scheme, MScheme::Eventbrite);
        assert_eq!(m.event_ids[0].value, "123456789");
    }
}
