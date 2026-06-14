//! The [`Event`] domain model, aligned with <https://schema.org/Event>.
//!
//! An event is a time-bounded occurrence (conference, appointment,
//! shift, encounter, sale, screening, …) with a location, parties,
//! optional offers, and links to other events.
//!
//! [`Event`] is the aggregate root of the whole crate: the matching,
//! search, validation, and privacy layers all operate on it. Only
//! [`Event::start_date`] and [`Event::name`] are conceptually required;
//! everything else defaults to empty or `None` via [`Event::new`].
//!
//! # Examples
//!
//! ```
//! use event_service::models::Event;
//!
//! use chrono::{TimeZone, Utc};
//! let start = Utc.with_ymd_and_hms(2026, 6, 1, 9, 0, 0).unwrap();
//! let mut event = Event::new("Summer Festival", start);
//! event.end_date = Some(start + chrono::Duration::hours(8));
//! event.keywords.push("music".into());
//! assert_eq!(event.name, "Summer Festival");
//! assert!(event.identifiers.is_empty());
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{
    EventAttendanceMode, EventStatus, EventType, Identifier, Location, Offer, Party, Reference,
};

/// An event resource. Mirrors schema.org/Event.
///
/// The full schema.org/Event has 40+ properties; this struct includes the
/// ones the service stores, indexes, and matches on. Property names use
/// `snake_case` per Rust convention (e.g. `start_date` for schema.org
/// `startDate`).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Event {
    /// Unique event identifier (internal UUID).
    pub id: Uuid,

    /// External identifiers (booking number, ticket, confirmation
    /// code, encounter ID, etc.).
    #[serde(default)]
    pub identifiers: Vec<Identifier>,

    /// `false` after soft-delete or when an event is cancelled and
    /// the caller wants to filter it out of default listings.
    pub active: bool,

    // -----------------------------------------------------------------------
    // schema.org/Thing properties
    // -----------------------------------------------------------------------
    /// Human-readable title (schema.org/name).
    pub name: String,

    /// Aliases (schema.org/alternateName).
    #[serde(default)]
    pub alternate_names: Vec<String>,

    /// Long-form description (schema.org/description).
    pub description: Option<String>,

    /// Short distinguishing description
    /// (schema.org/disambiguatingDescription).
    pub disambiguating_description: Option<String>,

    /// Canonical URL for this event (schema.org/url).
    pub url: Option<String>,

    /// Image URLs (schema.org/image).
    #[serde(default)]
    pub image: Vec<String>,

    /// URLs that unambiguously refer to the same event in other
    /// systems (schema.org/sameAs).
    #[serde(default)]
    pub same_as: Vec<String>,

    /// Descriptive tags (schema.org/keywords).
    #[serde(default)]
    pub keywords: Vec<String>,

    // -----------------------------------------------------------------------
    // Time window (schema.org/Event time properties)
    // -----------------------------------------------------------------------
    /// When the event starts (schema.org/startDate). Required.
    pub start_date: DateTime<Utc>,

    /// When the event ends (schema.org/endDate). When absent, the
    /// event is open-ended.
    pub end_date: Option<DateTime<Utc>>,

    /// When the doors open / admission begins (schema.org/doorTime).
    pub door_time: Option<DateTime<Utc>>,

    /// ISO 8601 duration (e.g. "PT1H30M") if no end date is recorded
    /// (schema.org/duration).
    pub duration: Option<String>,

    /// The originally scheduled start date if the event was
    /// rescheduled (schema.org/previousStartDate).
    pub previous_start_date: Option<DateTime<Utc>>,

    /// IANA time-zone name for display (e.g. "America/Los_Angeles").
    /// Storage is always UTC.
    pub time_zone: Option<String>,

    /// All-day event marker — when `true`, the time component of
    /// `start_date`/`end_date` is meaningless.
    #[serde(default)]
    pub all_day: bool,

    // -----------------------------------------------------------------------
    // Status, mode, type
    // -----------------------------------------------------------------------
    /// Lifecycle status (schema.org/eventStatus).
    pub event_status: EventStatus,
    /// How attendees join (schema.org/eventAttendanceMode).
    pub event_attendance_mode: EventAttendanceMode,
    /// Event subtype classification (local enum).
    pub event_type: EventType,

    // -----------------------------------------------------------------------
    // Capacity & accessibility
    // -----------------------------------------------------------------------
    /// schema.org/typicalAgeRange (e.g. "7-9", "18+").
    pub typical_age_range: Option<String>,

    /// schema.org/inLanguage — ISO 639-1 codes.
    #[serde(default)]
    pub in_language: Vec<String>,

    /// schema.org/isAccessibleForFree.
    pub is_accessible_for_free: Option<bool>,

    /// schema.org/maximumAttendeeCapacity (total).
    pub maximum_attendee_capacity: Option<u32>,

    /// schema.org/maximumPhysicalAttendeeCapacity.
    pub maximum_physical_attendee_capacity: Option<u32>,

    /// schema.org/maximumVirtualAttendeeCapacity.
    pub maximum_virtual_attendee_capacity: Option<u32>,

    /// schema.org/remainingAttendeeCapacity (places left).
    pub remaining_attendee_capacity: Option<u32>,

    // -----------------------------------------------------------------------
    // Location (where)
    // -----------------------------------------------------------------------
    /// schema.org/location. An event may have several locations
    /// (e.g. a primary venue plus a livestream URL).
    #[serde(default)]
    pub location: Vec<Location>,

    // -----------------------------------------------------------------------
    // Parties (who)
    // -----------------------------------------------------------------------
    /// schema.org/organizer.
    #[serde(default)]
    pub organizers: Vec<Party>,

    /// schema.org/performer.
    #[serde(default)]
    pub performers: Vec<Party>,

    /// schema.org/attendee.
    #[serde(default)]
    pub attendees: Vec<Party>,

    /// schema.org/sponsor.
    #[serde(default)]
    pub sponsors: Vec<Party>,

    /// schema.org/funder.
    #[serde(default)]
    pub funders: Vec<Party>,

    /// schema.org/contributor.
    #[serde(default)]
    pub contributors: Vec<Party>,

    // -----------------------------------------------------------------------
    // Subject matter / works
    // -----------------------------------------------------------------------
    /// schema.org/about — what the event is about.
    #[serde(default)]
    pub about: Vec<Reference>,

    /// schema.org/workFeatured / workPerformed (collapsed for
    /// simplicity — disambiguate via `Reference::kind`).
    #[serde(default)]
    pub works: Vec<Reference>,

    // -----------------------------------------------------------------------
    // Hierarchy
    // -----------------------------------------------------------------------
    /// schema.org/superEvent — the event this is part of.
    pub super_event: Option<Uuid>,

    /// schema.org/subEvent — events that are part of this one.
    #[serde(default)]
    pub sub_events: Vec<Uuid>,

    // -----------------------------------------------------------------------
    // Offers (pricing / tickets)
    // -----------------------------------------------------------------------
    /// schema.org/offers.
    #[serde(default)]
    pub offers: Vec<Offer>,

    // -----------------------------------------------------------------------
    // Cross-event links (merge / refer / see-also)
    // -----------------------------------------------------------------------
    /// Typed links to other events (merge `Replaces`/`ReplacedBy`,
    /// `Refer`, `Seealso`).
    #[serde(default)]
    pub links: Vec<EventLink>,

    // -----------------------------------------------------------------------
    // Audit timestamps
    // -----------------------------------------------------------------------
    /// When this record was first created (set by [`Event::new`]).
    pub created_at: DateTime<Utc>,
    /// When this record was last modified.
    pub updated_at: DateTime<Utc>,
}

/// A typed link from one event to another (merge / refer / see-also).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EventLink {
    /// The ID of the event being linked to.
    pub other_event_id: Uuid,
    /// The semantics of the link.
    pub link_type: LinkType,
}

/// The semantics of an [`EventLink`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    /// This event has been replaced by the linked event (e.g. after merge).
    ReplacedBy,
    /// This event replaces the linked event (the surviving record).
    Replaces,
    /// This event refers to the same real-world occurrence as the
    /// linked event (cross-system alias without merge).
    Refer,
    /// Loosely related event (suggestion / see-also).
    Seealso,
}

impl Event {
    /// Construct a new event with required fields. All optional
    /// fields default to empty / `None`; `id` is generated (UUID v4)
    /// and `created_at` / `updated_at` are set to now.
    ///
    /// `active` is `true` and the status / mode / type take their
    /// [`Default`] values (`Scheduled` / `Offline` / `Generic`).
    ///
    /// # Examples
    ///
    /// ```
    /// use event_service::models::Event;
    /// use event_service::models::{EventStatus, EventAttendanceMode, EventType};
    ///
    /// use chrono::{TimeZone, Utc};
    /// let start = Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap();
    /// let event = Event::new("Annual Conference", start);
    /// assert!(event.active);
    /// assert_eq!(event.event_status, EventStatus::Scheduled);
    /// assert_eq!(event.event_attendance_mode, EventAttendanceMode::Offline);
    /// assert_eq!(event.event_type, EventType::Generic);
    /// assert!(event.end_date.is_none());
    /// ```
    pub fn new(name: impl Into<String>, start_date: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            identifiers: Vec::new(),
            active: true,
            name: name.into(),
            alternate_names: Vec::new(),
            description: None,
            disambiguating_description: None,
            url: None,
            image: Vec::new(),
            same_as: Vec::new(),
            keywords: Vec::new(),
            start_date,
            end_date: None,
            door_time: None,
            duration: None,
            previous_start_date: None,
            time_zone: None,
            all_day: false,
            event_status: EventStatus::default(),
            event_attendance_mode: EventAttendanceMode::default(),
            event_type: EventType::default(),
            typical_age_range: None,
            in_language: Vec::new(),
            is_accessible_for_free: None,
            maximum_attendee_capacity: None,
            maximum_physical_attendee_capacity: None,
            maximum_virtual_attendee_capacity: None,
            remaining_attendee_capacity: None,
            location: Vec::new(),
            organizers: Vec::new(),
            performers: Vec::new(),
            attendees: Vec::new(),
            sponsors: Vec::new(),
            funders: Vec::new(),
            contributors: Vec::new(),
            about: Vec::new(),
            works: Vec::new(),
            super_event: None,
            sub_events: Vec::new(),
            offers: Vec::new(),
            links: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// First identifier value whose type matches `kind`, if any.
    ///
    /// Used to pull out a strong identifier (booking / ticket /
    /// confirmation / encounter / transaction) for deterministic
    /// matching and for display.
    ///
    /// # Examples
    ///
    /// ```
    /// use event_service::models::{Event, Identifier, IdentifierType};
    ///
    /// use chrono::{TimeZone, Utc};
    /// let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    /// let mut event = Event::new("Show", start);
    /// event.identifiers.push(Identifier::new(
    ///     IdentifierType::TicketNumber, "box-office".into(), "T-42".into(),
    /// ));
    /// assert_eq!(event.identifier_value(IdentifierType::TicketNumber), Some("T-42"));
    /// assert_eq!(event.identifier_value(IdentifierType::BookingNumber), None);
    /// ```
    pub fn identifier_value(&self, kind: super::IdentifierType) -> Option<&str> {
        self.identifiers
            .iter()
            .find(|id| id.identifier_type == kind)
            .map(|id| id.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Address, Location, Party, PartyKind, Place, VirtualLocation};
    use chrono::TimeZone;

    /// A fixed timestamp used across the tests for reproducibility.
    fn jan_2026() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 15, 9, 0, 0).unwrap()
    }

    /// `new` leaves all collections empty and applies enum defaults.
    #[test]
    fn new_event_defaults() {
        let event = Event::new("Annual Conference", jan_2026());
        assert!(event.active);
        assert_eq!(event.name, "Annual Conference");
        assert_eq!(event.event_status, EventStatus::Scheduled);
        assert_eq!(event.event_attendance_mode, EventAttendanceMode::Offline);
        assert_eq!(event.event_type, EventType::Generic);
        assert!(event.identifiers.is_empty());
        assert!(event.location.is_empty());
        assert!(event.organizers.is_empty());
        assert!(event.end_date.is_none());
    }

    /// A fully populated event survives a JSON round-trip unchanged.
    #[test]
    fn roundtrip_serde() {
        let mut event = Event::new("Concert", jan_2026());
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
            email: None,
            url: None,
        });
        event.keywords.push("music".into());
        event.in_language.push("en".into());

        let json = serde_json::to_string(&event).unwrap();
        let back: Event = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Concert");
        assert_eq!(back.event_type, EventType::Music);
        assert_eq!(back.location.len(), 2);
        assert_eq!(back.organizers.len(), 1);
        assert_eq!(back.keywords, vec!["music".to_string()]);
    }

    /// Every `Location` variant serializes via its `kind` tag and
    /// deserializes back to the same value.
    #[test]
    fn location_variants_roundtrip() {
        // Each Location variant should serialize via its `kind` tag.
        let cases = vec![
            Location::Text {
                value: "TBA".into(),
            },
            Location::Virtual(VirtualLocation {
                name: None,
                url: "https://example.test/zoom".into(),
            }),
            Location::PostalAddress(Address {
                use_type: None,
                line1: Some("1 Infinite Loop".into()),
                line2: None,
                city: Some("Cupertino".into()),
                state: Some("CA".into()),
                postal_code: Some("95014".into()),
                country: Some("US".into()),
            }),
        ];
        for loc in cases {
            let json = serde_json::to_string(&loc).unwrap();
            let back: Location = serde_json::from_str(&json).unwrap();
            assert_eq!(back, loc);
        }
    }
}
