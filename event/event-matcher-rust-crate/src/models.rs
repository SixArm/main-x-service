//! Data models for events, following schema.org/Event.
//!
//! This module is intentionally **logic-free**: it defines the types that
//! flow through the matching engine but contains no matching code itself.
//! See [`crate::matcher`] for the engine and [`crate::normalizer`] for the
//! text transformations that the matcher applies to these fields.
//!
//! The shape of [`Event`] mirrors the properties listed at
//! <https://schema.org/Event>. Field names use Rust conventions
//! (`snake_case`) but the semantics match the schema.org definitions one
//! for one — a record produced from a `schema:Event` JSON-LD payload can
//! be loaded with a thin field-by-field mapping.
//!
//! All public types here are `Serialize + Deserialize` so they round-trip
//! through JSON, `MessagePack`, or any other `serde` format.
//!
//! ## Building an event
//!
//! Prefer [`Event::builder`] over constructing the struct literal — the
//! builder accepts `impl Into<String>` so call-sites can pass `&str`,
//! `String`, or owned values interchangeably.
//!
//! ```
//! use event_matcher::Event;
//!
//! let e = Event::builder()
//!     .name("Glastonbury Festival 2024")
//!     .add_alternate_name("Glasto 2024")
//!     .start_date("2024-06-26T09:00:00Z")
//!     .end_date("2024-06-30T23:59:00Z")
//!     .build();
//!
//! assert_eq!(e.name.as_deref(), Some("Glastonbury Festival 2024"));
//! ```

use serde::{Deserialize, Serialize};

/// Postal address used as supporting evidence in event matcher.
///
/// All fields are `Option<String>` so partial addresses are first-class —
/// a record with only a postcode is still useful for matching.
///
/// The matcher does **not** weight every component equally; see
/// [`crate::matcher::MatchingEngine`] for the weighted comparison rules.
///
/// # Example
///
/// ```
/// use event_matcher::Address;
///
/// let mut addr = Address::new();
/// addr.line1    = Some("Worthy Farm".into());
/// addr.city     = Some("Pilton".into());
/// addr.postcode = Some("BA4 4BY".into());
///
/// assert_eq!(addr.postcode.as_deref(), Some("BA4 4BY"));
/// assert!(addr.country.is_none());
/// ```
///
/// `Address` is JSON round-trippable.
///
/// ```
/// # use event_matcher::Address;
/// let mut a = Address::new();
/// a.postcode = Some("BA4 4BY".into());
///
/// let json = serde_json::to_string(&a).unwrap();
/// let back: Address = serde_json::from_str(&json).unwrap();
/// assert_eq!(a, back);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Address {
    /// First line — typically house number and street, e.g. `"Worthy Farm"`.
    pub line1: Option<String>,
    /// Second line — typically flat, locality, or care-of details.
    pub line2: Option<String>,
    /// Town or city, e.g. `"Pilton"`.
    pub city: Option<String>,
    /// County or administrative region, e.g. `"Somerset"`.
    pub county: Option<String>,
    /// Postal code, e.g. `"BA4 4BY"`. Compared after whitespace normalisation.
    pub postcode: Option<String>,
    /// Country, e.g. `"England"` or `"United Kingdom"`.
    pub country: Option<String>,
}

impl Address {
    /// Construct an empty address with every field set to `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use event_matcher::Address;
    ///
    /// let a = Address::new();
    /// assert!(a.line1.is_none());
    /// assert!(a.postcode.is_none());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            line1: None,
            line2: None,
            city: None,
            county: None,
            postcode: None,
            country: None,
        }
    }

    /// Fluent setter for `line1`.
    #[must_use]
    pub fn with_line1(mut self, value: impl Into<String>) -> Self {
        self.line1 = Some(value.into());
        self
    }

    /// Fluent setter for `line2`.
    #[must_use]
    pub fn with_line2(mut self, value: impl Into<String>) -> Self {
        self.line2 = Some(value.into());
        self
    }

    /// Fluent setter for `city`.
    #[must_use]
    pub fn with_city(mut self, value: impl Into<String>) -> Self {
        self.city = Some(value.into());
        self
    }

    /// Fluent setter for `county`.
    #[must_use]
    pub fn with_county(mut self, value: impl Into<String>) -> Self {
        self.county = Some(value.into());
        self
    }

    /// Fluent setter for `postcode`.
    #[must_use]
    pub fn with_postcode(mut self, value: impl Into<String>) -> Self {
        self.postcode = Some(value.into());
        self
    }

    /// Fluent setter for `country`.
    #[must_use]
    pub fn with_country(mut self, value: impl Into<String>) -> Self {
        self.country = Some(value.into());
        self
    }
}

impl Default for Address {
    fn default() -> Self {
        Self::new()
    }
}

/// Where an event takes place — corresponds to schema.org `Event.location`.
///
/// Schema.org accepts `Place`, `PostalAddress`, `Text`, or `VirtualLocation`
/// as a value for `Event.location`. This crate models all four flavours
/// inside one struct so a single `Location` can describe a purely
/// physical venue, a purely virtual one, or a hybrid:
///
/// - `venue_name` carries the textual name of the venue (`Place.name`),
///   e.g. `"Worthy Farm"` or `"Madison Square Garden"`.
/// - `address` carries the postal address (`PostalAddress`).
/// - `latitude` / `longitude` carry geographic coordinates (`Place.geo`).
/// - `virtual_url` carries the join URL for `VirtualLocation`.
///
/// Every field is optional. A `Location` with all fields `None` is
/// equivalent to no location at all and is skipped by the matcher.
///
/// # Example
///
/// ```
/// use event_matcher::{Address, Location};
///
/// let l = Location::new()
///     .with_venue_name("Worthy Farm")
///     .with_address(Address::new().with_postcode("BA4 4BY"))
///     .with_latitude(51.150_3)
///     .with_longitude(-2.586_2);
///
/// assert_eq!(l.venue_name.as_deref(), Some("Worthy Farm"));
/// assert_eq!(l.latitude_as_decimal_degrees, Some(51.150_3));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Location {
    /// Textual name of the venue, e.g. `"Madison Square Garden"`.
    pub venue_name: Option<String>,
    /// Postal address of the physical venue.
    pub address: Option<Address>,
    /// Latitude in decimal degrees. Conventionally `[-90.0, 90.0]`.
    /// Values outside that range or non-finite values are stored as
    /// supplied for round-trip honesty, but the coordinates scorer treats
    /// them as missing.
    pub latitude_as_decimal_degrees: Option<f64>,
    /// Longitude in decimal degrees. Conventionally `[-180.0, 180.0]`.
    pub longitude_as_decimal_degrees: Option<f64>,
    /// Join URL for a virtual event (schema.org `VirtualLocation.url`),
    /// e.g. `"https://zoom.us/j/123456"`. Compared by exact equality
    /// after trimming whitespace.
    pub virtual_url: Option<String>,
}

impl Location {
    /// Construct an empty location with every field set to `None`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            venue_name: None,
            address: None,
            latitude_as_decimal_degrees: None,
            longitude_as_decimal_degrees: None,
            virtual_url: None,
        }
    }

    /// Fluent setter for `venue_name`.
    #[must_use]
    pub fn with_venue_name(mut self, value: impl Into<String>) -> Self {
        self.venue_name = Some(value.into());
        self
    }

    /// Fluent setter for `address`.
    #[must_use]
    pub fn with_address(mut self, value: Address) -> Self {
        self.address = Some(value);
        self
    }

    /// Fluent setter for `latitude`.
    #[must_use]
    pub fn with_latitude(mut self, value: f64) -> Self {
        self.latitude_as_decimal_degrees = Some(value);
        self
    }

    /// Fluent setter for `longitude`.
    #[must_use]
    pub fn with_longitude(mut self, value: f64) -> Self {
        self.longitude_as_decimal_degrees = Some(value);
        self
    }

    /// Fluent setter for `virtual_url`.
    #[must_use]
    pub fn with_virtual_url(mut self, value: impl Into<String>) -> Self {
        self.virtual_url = Some(value.into());
        self
    }
}

impl Default for Location {
    fn default() -> Self {
        Self::new()
    }
}

/// Schema.org Event subtypes — the kind of event.
///
/// The variants mirror the direct subtypes of `schema:Event` listed at
/// <https://schema.org/Event>. The catch-all [`EventCategory::Other`]
/// variant lets callers express categories that the crate does not
/// enumerate; two `Other` values match iff their carried string is
/// structurally equal (`PartialEq`).
///
/// The enum is `#[non_exhaustive]` so future variants can be added without
/// breaking `SemVer` for downstream pattern matches.
///
/// # Example
///
/// ```
/// use event_matcher::EventCategory;
///
/// assert_eq!(EventCategory::MusicEvent, EventCategory::MusicEvent);
/// assert_ne!(EventCategory::MusicEvent, EventCategory::ComedyEvent);
/// assert_eq!(
///     EventCategory::Other("WeddingEvent".into()),
///     EventCategory::Other("WeddingEvent".into())
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EventCategory {
    /// Business event: trade show, networking, professional gathering.
    BusinessEvent,
    /// Event aimed primarily at children.
    ChildrensEvent,
    /// Stand-up comedy or comedic performance.
    ComedyEvent,
    /// Multi-session conference (schema.org `ConferenceEvent`).
    ConferenceEvent,
    /// Specific delivery of a course (schema.org `CourseInstance`).
    CourseInstance,
    /// Dance performance or social dance.
    DanceEvent,
    /// Delivery of goods to a customer (schema.org `DeliveryEvent`).
    DeliveryEvent,
    /// General educational event: lecture, workshop, training session.
    EducationEvent,
    /// Recurring series sharing an identity (schema.org `EventSeries`).
    EventSeries,
    /// Exhibition such as a museum show or trade exhibition.
    ExhibitionEvent,
    /// Festival, fair, or similar multi-attraction celebration.
    Festival,
    /// Food-centred event: tasting, supper club, food fair.
    FoodEvent,
    /// Hackathon or coding marathon.
    Hackathon,
    /// Literary event: reading, book launch, signing.
    LiteraryEvent,
    /// Concert, recital, or other musical performance.
    MusicEvent,
    /// Performing-arts event (catch-all for live performance).
    PerformingArtsEvent,
    /// Release of a creative work (schema.org `PublicationEvent`).
    PublicationEvent,
    /// Sale, auction, or other commercial offer-based event.
    SaleEvent,
    /// Screening of a film or video work.
    ScreeningEvent,
    /// Social event: party, mixer, meetup.
    SocialEvent,
    /// Sporting event or competition.
    SportsEvent,
    /// Stage play or other theatrical performance.
    TheaterEvent,
    /// Visual-arts event: gallery opening, art-walk.
    VisualArtsEvent,
    /// Catch-all for categories not enumerated above. The carried string
    /// participates in equality, so `Other("foo")` does not match
    /// `Other("bar")`.
    Other(String),
}

/// Schema.org `EventStatusType` — the lifecycle state of an event.
///
/// The variants are the five enumeration values defined at
/// <https://schema.org/EventStatusType>.
///
/// The enum is `#[non_exhaustive]` so future variants can be added without
/// breaking `SemVer` for downstream pattern matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EventStatus {
    /// `schema:EventScheduled` — the event is confirmed and on track.
    EventScheduled,
    /// `schema:EventCancelled` — the event will not take place.
    EventCancelled,
    /// `schema:EventPostponed` — the event has been postponed, no new date yet.
    EventPostponed,
    /// `schema:EventRescheduled` — the event has been moved to a new date.
    EventRescheduled,
    /// `schema:EventMovedOnline` — the event has moved to a virtual format.
    EventMovedOnline,
}

/// Schema.org `EventAttendanceModeEnumeration` — physical, virtual, or both.
///
/// Defined at <https://schema.org/EventAttendanceModeEnumeration>.
///
/// The enum is `#[non_exhaustive]` so future variants can be added without
/// breaking `SemVer` for downstream pattern matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EventAttendanceMode {
    /// Physical attendance only.
    OfflineEventAttendanceMode,
    /// Virtual attendance only.
    OnlineEventAttendanceMode,
    /// Hybrid event with both physical and virtual attendance.
    MixedEventAttendanceMode,
}

/// Issuer / scheme of an [`EventId`].
///
/// Identifiers from different schemes never match each other, even when
/// their string values happen to coincide — see [`EventId`].
///
/// The enum is `#[non_exhaustive]` so future variants can be added without
/// breaking `SemVer` for downstream pattern matches.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EventIdScheme {
    /// Wikidata QID for the event (e.g. `Q15290`).
    Wikidata,
    /// Eventbrite event identifier.
    Eventbrite,
    /// Meetup.com event identifier.
    Meetup,
    /// Ticketmaster event identifier.
    Ticketmaster,
    /// Songkick concert / festival identifier.
    Songkick,
    /// Bandsintown event identifier.
    Bandsintown,
    /// Facebook event identifier.
    Facebook,
    /// Luma (lu.ma) event identifier.
    Luma,
    /// Google Calendar event identifier.
    GoogleCalendar,
    /// iCalendar `UID` property (RFC 5545).
    ICalendarUid,
    /// Catch-all for schemes not enumerated above. The carried string
    /// participates in equality.
    Other(String),
}

/// External identifier for an event, scoped by its issuing scheme.
///
/// Two `EventId` values are equal iff both their [`EventIdScheme`] and
/// their `value` are equal. Equality is structural — no per-scheme
/// canonicalisation is performed.
///
/// # Example
///
/// ```
/// use event_matcher::{EventId, EventIdScheme};
///
/// let a = EventId::new(EventIdScheme::Eventbrite, "123456789").unwrap();
/// let b = EventId::new(EventIdScheme::Eventbrite, " 123456789 ").unwrap();
/// assert_eq!(a, b, "values are trimmed at construction");
/// assert!(EventId::new(EventIdScheme::Eventbrite, "").is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId {
    /// The issuing scheme.
    pub scheme: EventIdScheme,
    /// The identifier value, trimmed of surrounding whitespace.
    pub value: String,
}

impl EventId {
    /// Construct an [`EventId`], trimming the value of surrounding
    /// whitespace. Returns `None` if the trimmed value is empty.
    ///
    /// No further normalisation is applied — different schemes have
    /// different rules and the crate makes no assumptions.
    ///
    /// # Example
    ///
    /// ```
    /// use event_matcher::{EventId, EventIdScheme};
    ///
    /// let id = EventId::new(EventIdScheme::Wikidata, "  Q15290  ").unwrap();
    /// assert_eq!(id.value, "Q15290");
    ///
    /// assert!(EventId::new(EventIdScheme::Wikidata, "   ").is_none());
    /// ```
    pub fn new(scheme: EventIdScheme, value: impl Into<String>) -> Option<Self> {
        let trimmed = value.into().trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self {
                scheme,
                value: trimmed,
            })
        }
    }
}

/// Core event data structure — corresponds to schema.org `Event`.
///
/// Every field is optional (or defaults to empty). The matcher tolerates
/// missing data field-by-field — a `None` value never penalises an event.
/// See [`crate::matcher::MatchingEngine::match_events`] for how missing
/// fields affect the weighted score.
///
/// Construct via [`Event::builder`] rather than struct literal syntax so
/// the call-site stays compact and forward-compatible if fields are added.
///
/// # Field mapping to schema.org
///
/// | Field | Schema.org property |
/// |---|---|
/// | `name` | `schema:name` |
/// | `alternate_names` | `schema:alternateName` |
/// | `description` | `schema:description` |
/// | `url` | `schema:url` |
/// | `event_ids` | `schema:identifier` |
/// | `category` | (closest match to the subtype of `schema:Event`) |
/// | `keywords` | `schema:keywords` |
/// | `in_language` | `schema:inLanguage` |
/// | `start_date` | `schema:startDate` |
/// | `end_date` | `schema:endDate` |
/// | `door_time` | `schema:doorTime` |
/// | `previous_start_date` | `schema:previousStartDate` |
/// | `event_status` | `schema:eventStatus` |
/// | `event_attendance_mode` | `schema:eventAttendanceMode` |
/// | `location` | `schema:location` |
/// | `organizer` | `schema:organizer` |
/// | `performers` | `schema:performer` |
/// | `maximum_attendee_capacity` | `schema:maximumAttendeeCapacity` |
/// | `maximum_physical_attendee_capacity` | `schema:maximumPhysicalAttendeeCapacity` |
/// | `maximum_virtual_attendee_capacity` | `schema:maximumVirtualAttendeeCapacity` |
/// | `is_accessible_for_free` | `schema:isAccessibleForFree` |
/// | `typical_age_range` | `schema:typicalAgeRange` |
/// | `super_event_id` | `schema:superEvent` (id only) |
///
/// # Example
///
/// ```
/// use event_matcher::Event;
///
/// let e = Event::builder()
///     .name("RustConf 2024")
///     .add_alternate_name("RustConf '24")
///     .start_date("2024-09-10T09:00:00Z")
///     .end_date("2024-09-13T17:00:00Z")
///     .organizer("Rust Foundation")
///     .build();
///
/// assert_eq!(e.name.as_deref(), Some("RustConf 2024"));
/// assert_eq!(e.alternate_names, vec!["RustConf '24".to_string()]);
/// ```
///
/// `Event` round-trips through `serde`.
///
/// ```
/// # use event_matcher::Event;
/// let e = Event::builder().name("RustConf").build();
/// let json = serde_json::to_string(&e).unwrap();
/// let back: Event = serde_json::from_str(&json).unwrap();
/// assert_eq!(e, back);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Event {
    /// Primary canonical name of the event, e.g. `"Glastonbury Festival 2024"`.
    pub name: Option<String>,

    /// Aliases, translations, or alternative titles of the event, e.g.
    /// `vec!["Glasto 2024".into()]`. Default empty.
    pub alternate_names: Vec<String>,

    /// Free-text description of the event (`schema:description`).
    pub description: Option<String>,

    /// Canonical URL for the event (`schema:url`).
    pub url: Option<String>,

    /// External scheme-scoped identifiers for the event. Sharing any one
    /// `(scheme, value)` pair across two events is a deterministic match.
    pub event_ids: Vec<EventId>,

    /// Local identifier issued by the originating system. Not normalised —
    /// different organisations may issue colliding values.
    pub local_id: Option<String>,

    /// Coarse-grained category of the event (see [`EventCategory`]).
    pub category: Option<EventCategory>,

    /// Free-form descriptive tags (`schema:keywords`).
    pub keywords: Vec<String>,

    /// Content language as an IETF BCP-47 tag, e.g. `"en-GB"`, `"fr"`.
    /// Stored as supplied; compared case-insensitively after trim.
    pub in_language: Option<String>,

    /// Expected demographic age range, e.g. `"7-9"` (`schema:typicalAgeRange`).
    pub typical_age_range: Option<String>,

    /// Start date or date-time in ISO 8601 form. Examples:
    /// `"2024-06-26"`, `"2024-06-26T09:00:00Z"`, `"2024-06-26T09:00:00+01:00"`.
    pub start_date: Option<String>,

    /// End date or date-time in ISO 8601 form.
    pub end_date: Option<String>,

    /// Door-opening time (`schema:doorTime`) in ISO 8601 form.
    pub door_time: Option<String>,

    /// Original start date prior to rescheduling (`schema:previousStartDate`).
    pub previous_start_date: Option<String>,

    /// Current status — scheduled, cancelled, postponed, rescheduled, or
    /// moved online (see [`EventStatus`]).
    pub event_status: Option<EventStatus>,

    /// Physical, virtual, or mixed (see [`EventAttendanceMode`]).
    pub event_attendance_mode: Option<EventAttendanceMode>,

    /// Where the event takes place — venue, address, coordinates, and/or
    /// virtual URL (see [`Location`]).
    pub location: Option<Location>,

    /// Country code in ISO 3166-1 alpha-2 form, e.g. `"GB"`, `"US"`,
    /// `"FR"`. Convenient quick lookup when [`Location::address`] is
    /// absent. Stored as supplied — no canonicalisation is performed at
    /// construction time. The matcher compares case-insensitively after
    /// trimming whitespace.
    pub country_code_as_iso_3166_1_alpha_2: Option<String>,

    /// Name of the organiser (person or organisation). Single string for
    /// simplicity; callers needing multiple organisers should join them
    /// with `", "` or pre-pick a canonical organiser.
    pub organizer: Option<String>,

    /// Performer names — actors, bands, speakers, athletes
    /// (`schema:performer`). Compared as a best-of cartesian product.
    pub performers: Vec<String>,

    /// Total expected capacity (`schema:maximumAttendeeCapacity`).
    pub maximum_attendee_capacity: Option<u32>,

    /// Offline-attendance capacity (`schema:maximumPhysicalAttendeeCapacity`).
    pub maximum_physical_attendee_capacity: Option<u32>,

    /// Online-attendance capacity (`schema:maximumVirtualAttendeeCapacity`).
    pub maximum_virtual_attendee_capacity: Option<u32>,

    /// True if the event is free to attend (`schema:isAccessibleForFree`).
    pub is_accessible_for_free: Option<bool>,

    /// Identifier of the parent event (`schema:superEvent`). Stored as a
    /// free-form string so callers can carry whichever scheme they need.
    pub super_event_id: Option<String>,
}

impl Event {
    /// Begin constructing an [`Event`] with the [`EventBuilder`].
    ///
    /// All fields default to `None` / empty until a setter is called.
    ///
    /// # Example
    ///
    /// ```
    /// use event_matcher::Event;
    ///
    /// let e = Event::builder()
    ///     .name("RustConf 2024")
    ///     .build();
    ///
    /// assert_eq!(e.name.as_deref(), Some("RustConf 2024"));
    /// ```
    #[must_use]
    pub fn builder() -> EventBuilder {
        EventBuilder::default()
    }

    /// Validate that the event carries a primary name.
    ///
    /// Returns `Ok(())` if `name` is set.
    ///
    /// This is **not** invoked automatically by the matcher — call it at the
    /// system boundary when you ingest data, not on every comparison.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MatchingError::MissingField`] if `name` is `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use event_matcher::Event;
    ///
    /// assert!(Event::builder().name("RustConf").build().validate().is_ok());
    /// assert!(Event::builder().build().validate().is_err());
    /// ```
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.is_none() {
            return Err(crate::MatchingError::MissingField(
                "name is required".to_string(),
            ));
        }
        Ok(())
    }
}

/// Fluent builder for [`Event`].
///
/// All string setters accept `impl Into<String>` so call-sites may pass
/// `&str`, `String`, or `&String` interchangeably without explicit
/// conversion.
///
/// # Example
///
/// ```
/// use event_matcher::{Event, EventBuilder, EventCategory};
///
/// let e: Event = EventBuilder::default()
///     .name(String::from("RustConf 2024"))
///     .add_alternate_name("RustConf '24")
///     .start_date("2024-09-10T09:00:00Z")
///     .category(EventCategory::ConferenceEvent)
///     .build();
///
/// assert_eq!(e.name.as_deref(), Some("RustConf 2024"));
/// assert_eq!(e.category, Some(EventCategory::ConferenceEvent));
/// ```
#[derive(Default)]
pub struct EventBuilder {
    name: Option<String>,
    alternate_names: Vec<String>,
    description: Option<String>,
    url: Option<String>,
    event_ids: Vec<EventId>,
    local_id: Option<String>,
    category: Option<EventCategory>,
    keywords: Vec<String>,
    in_language: Option<String>,
    typical_age_range: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
    door_time: Option<String>,
    previous_start_date: Option<String>,
    event_status: Option<EventStatus>,
    event_attendance_mode: Option<EventAttendanceMode>,
    location: Option<Location>,
    country_code_as_iso_3166_1_alpha_2: Option<String>,
    organizer: Option<String>,
    performers: Vec<String>,
    maximum_attendee_capacity: Option<u32>,
    maximum_physical_attendee_capacity: Option<u32>,
    maximum_virtual_attendee_capacity: Option<u32>,
    is_accessible_for_free: Option<bool>,
    super_event_id: Option<String>,
}

impl EventBuilder {
    /// Set the primary canonical name.
    #[must_use]
    pub fn name<S: Into<String>>(mut self, value: S) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Replace the entire list of alternate names.
    #[must_use]
    pub fn alternate_names(mut self, value: Vec<String>) -> Self {
        self.alternate_names = value;
        self
    }

    /// Append a single alternate name.
    #[must_use]
    pub fn add_alternate_name<S: Into<String>>(mut self, value: S) -> Self {
        self.alternate_names.push(value.into());
        self
    }

    /// Set the free-text description.
    #[must_use]
    pub fn description<S: Into<String>>(mut self, value: S) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Set the canonical URL.
    #[must_use]
    pub fn url<S: Into<String>>(mut self, value: S) -> Self {
        self.url = Some(value.into());
        self
    }

    /// Replace the entire list of external event IDs.
    #[must_use]
    pub fn event_ids(mut self, value: Vec<EventId>) -> Self {
        self.event_ids = value;
        self
    }

    /// Append a single external event ID.
    #[must_use]
    pub fn add_event_id(mut self, value: EventId) -> Self {
        self.event_ids.push(value);
        self
    }

    /// Set the local identifier.
    #[must_use]
    pub fn local_id<S: Into<String>>(mut self, value: S) -> Self {
        self.local_id = Some(value.into());
        self
    }

    /// Set the event's category.
    #[must_use]
    pub fn category(mut self, value: EventCategory) -> Self {
        self.category = Some(value);
        self
    }

    /// Replace the entire list of keywords.
    #[must_use]
    pub fn keywords(mut self, value: Vec<String>) -> Self {
        self.keywords = value;
        self
    }

    /// Append a single keyword.
    #[must_use]
    pub fn add_keyword<S: Into<String>>(mut self, value: S) -> Self {
        self.keywords.push(value.into());
        self
    }

    /// Set the BCP-47 language tag.
    #[must_use]
    pub fn in_language<S: Into<String>>(mut self, value: S) -> Self {
        self.in_language = Some(value.into());
        self
    }

    /// Set the typical-age-range string.
    #[must_use]
    pub fn typical_age_range<S: Into<String>>(mut self, value: S) -> Self {
        self.typical_age_range = Some(value.into());
        self
    }

    /// Set the start date or date-time in ISO 8601 form.
    #[must_use]
    pub fn start_date<S: Into<String>>(mut self, value: S) -> Self {
        self.start_date = Some(value.into());
        self
    }

    /// Set the end date or date-time in ISO 8601 form.
    #[must_use]
    pub fn end_date<S: Into<String>>(mut self, value: S) -> Self {
        self.end_date = Some(value.into());
        self
    }

    /// Set the door-opening time in ISO 8601 form.
    #[must_use]
    pub fn door_time<S: Into<String>>(mut self, value: S) -> Self {
        self.door_time = Some(value.into());
        self
    }

    /// Set the previous-start-date for rescheduled events.
    #[must_use]
    pub fn previous_start_date<S: Into<String>>(mut self, value: S) -> Self {
        self.previous_start_date = Some(value.into());
        self
    }

    /// Set the event lifecycle status.
    #[must_use]
    pub fn event_status(mut self, value: EventStatus) -> Self {
        self.event_status = Some(value);
        self
    }

    /// Set the attendance mode (offline / online / mixed).
    #[must_use]
    pub fn event_attendance_mode(mut self, value: EventAttendanceMode) -> Self {
        self.event_attendance_mode = Some(value);
        self
    }

    /// Set the event's location.
    #[must_use]
    pub fn location(mut self, value: Location) -> Self {
        self.location = Some(value);
        self
    }

    /// Set the country code in ISO 3166-1 alpha-2 form.
    #[must_use]
    pub fn country_code_as_iso_3166_1_alpha_2<S: Into<String>>(mut self, value: S) -> Self {
        self.country_code_as_iso_3166_1_alpha_2 = Some(value.into());
        self
    }

    /// Set the organiser name.
    #[must_use]
    pub fn organizer<S: Into<String>>(mut self, value: S) -> Self {
        self.organizer = Some(value.into());
        self
    }

    /// Replace the entire list of performer names.
    #[must_use]
    pub fn performers(mut self, value: Vec<String>) -> Self {
        self.performers = value;
        self
    }

    /// Append a single performer name.
    #[must_use]
    pub fn add_performer<S: Into<String>>(mut self, value: S) -> Self {
        self.performers.push(value.into());
        self
    }

    /// Set the total expected attendee capacity.
    #[must_use]
    pub fn maximum_attendee_capacity(mut self, value: u32) -> Self {
        self.maximum_attendee_capacity = Some(value);
        self
    }

    /// Set the offline-attendance capacity.
    #[must_use]
    pub fn maximum_physical_attendee_capacity(mut self, value: u32) -> Self {
        self.maximum_physical_attendee_capacity = Some(value);
        self
    }

    /// Set the online-attendance capacity.
    #[must_use]
    pub fn maximum_virtual_attendee_capacity(mut self, value: u32) -> Self {
        self.maximum_virtual_attendee_capacity = Some(value);
        self
    }

    /// Set the "free to attend" flag.
    #[must_use]
    pub fn is_accessible_for_free(mut self, value: bool) -> Self {
        self.is_accessible_for_free = Some(value);
        self
    }

    /// Set the parent event identifier.
    #[must_use]
    pub fn super_event_id<S: Into<String>>(mut self, value: S) -> Self {
        self.super_event_id = Some(value.into());
        self
    }

    /// Consume the builder and produce the [`Event`].
    #[must_use]
    pub fn build(self) -> Event {
        Event {
            name: self.name,
            alternate_names: self.alternate_names,
            description: self.description,
            url: self.url,
            event_ids: self.event_ids,
            local_id: self.local_id,
            category: self.category,
            keywords: self.keywords,
            in_language: self.in_language,
            typical_age_range: self.typical_age_range,
            start_date: self.start_date,
            end_date: self.end_date,
            door_time: self.door_time,
            previous_start_date: self.previous_start_date,
            event_status: self.event_status,
            event_attendance_mode: self.event_attendance_mode,
            location: self.location,
            country_code_as_iso_3166_1_alpha_2: self.country_code_as_iso_3166_1_alpha_2,
            organizer: self.organizer,
            performers: self.performers,
            maximum_attendee_capacity: self.maximum_attendee_capacity,
            maximum_physical_attendee_capacity: self.maximum_physical_attendee_capacity,
            maximum_virtual_attendee_capacity: self.maximum_virtual_attendee_capacity,
            is_accessible_for_free: self.is_accessible_for_free,
            super_event_id: self.super_event_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_new_is_all_none() {
        let a = Address::new();
        assert!(a.line1.is_none());
        assert!(a.line2.is_none());
        assert!(a.city.is_none());
        assert!(a.county.is_none());
        assert!(a.postcode.is_none());
        assert!(a.country.is_none());
    }

    #[test]
    fn address_default_matches_new() {
        assert_eq!(Address::default(), Address::new());
    }

    #[test]
    fn address_fluent_builders_chain() {
        let a = Address::new()
            .with_line1("Worthy Farm")
            .with_city("Pilton")
            .with_postcode("BA4 4BY")
            .with_country("United Kingdom");
        assert_eq!(a.line1.as_deref(), Some("Worthy Farm"));
        assert_eq!(a.city.as_deref(), Some("Pilton"));
        assert_eq!(a.postcode.as_deref(), Some("BA4 4BY"));
        assert_eq!(a.country.as_deref(), Some("United Kingdom"));
        assert!(a.line2.is_none());
        assert!(a.county.is_none());
    }

    #[test]
    fn address_round_trips_through_serde() {
        let mut a = Address::new();
        a.line1 = Some("Worthy Farm".into());
        a.postcode = Some("BA4 4BY".into());
        let json = serde_json::to_string(&a).expect("serialise");
        let back: Address = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(a, back);
    }

    #[test]
    fn location_new_is_all_none() {
        let l = Location::new();
        assert!(l.venue_name.is_none());
        assert!(l.address.is_none());
        assert!(l.latitude_as_decimal_degrees.is_none());
        assert!(l.longitude_as_decimal_degrees.is_none());
        assert!(l.virtual_url.is_none());
    }

    #[test]
    fn location_default_matches_new() {
        assert_eq!(Location::default(), Location::new());
    }

    #[test]
    fn location_fluent_builders_chain() {
        let l = Location::new()
            .with_venue_name("Worthy Farm")
            .with_latitude(51.150_3)
            .with_longitude(-2.586_2);
        assert_eq!(l.venue_name.as_deref(), Some("Worthy Farm"));
        assert_eq!(l.latitude_as_decimal_degrees, Some(51.150_3));
        assert_eq!(l.longitude_as_decimal_degrees, Some(-2.586_2));
    }

    #[test]
    fn event_builder_starts_empty() {
        let e = Event::builder().build();
        assert!(e.name.is_none());
        assert!(e.alternate_names.is_empty());
        assert!(e.description.is_none());
        assert!(e.url.is_none());
        assert!(e.event_ids.is_empty());
        assert!(e.local_id.is_none());
        assert!(e.category.is_none());
        assert!(e.keywords.is_empty());
        assert!(e.in_language.is_none());
        assert!(e.start_date.is_none());
        assert!(e.end_date.is_none());
        assert!(e.location.is_none());
        assert!(e.organizer.is_none());
        assert!(e.performers.is_empty());
        assert!(e.maximum_attendee_capacity.is_none());
        assert!(e.is_accessible_for_free.is_none());
        assert!(e.super_event_id.is_none());
    }

    #[test]
    fn event_builder_accepts_all_fields() {
        let e = Event::builder()
            .name("RustConf 2024")
            .add_alternate_name("RustConf '24")
            .description("Annual Rust conference")
            .url("https://rustconf.com/2024")
            .start_date("2024-09-10T09:00:00Z")
            .end_date("2024-09-13T17:00:00Z")
            .door_time("2024-09-10T08:30:00Z")
            .organizer("Rust Foundation")
            .add_performer("Niko Matsakis")
            .category(EventCategory::ConferenceEvent)
            .event_status(EventStatus::EventScheduled)
            .event_attendance_mode(EventAttendanceMode::MixedEventAttendanceMode)
            .country_code_as_iso_3166_1_alpha_2("US")
            .maximum_attendee_capacity(1000)
            .is_accessible_for_free(false)
            .in_language("en")
            .typical_age_range("18-99")
            .add_keyword("rust")
            .build();
        assert_eq!(e.name.as_deref(), Some("RustConf 2024"));
        assert_eq!(e.alternate_names.len(), 1);
        assert_eq!(e.start_date.as_deref(), Some("2024-09-10T09:00:00Z"));
        assert_eq!(e.organizer.as_deref(), Some("Rust Foundation"));
        assert_eq!(e.category, Some(EventCategory::ConferenceEvent));
        assert_eq!(e.event_status, Some(EventStatus::EventScheduled));
        assert_eq!(e.maximum_attendee_capacity, Some(1000));
        assert_eq!(e.is_accessible_for_free, Some(false));
    }

    #[test]
    fn event_builder_accepts_str_and_string() {
        let e = Event::builder()
            .name("RustConf")
            .add_alternate_name(String::from("RustConf '24"))
            .build();
        assert_eq!(e.name.as_deref(), Some("RustConf"));
        assert_eq!(e.alternate_names, vec!["RustConf '24".to_string()]);
    }

    #[test]
    fn event_validate_requires_a_name() {
        assert!(Event::builder().name("RustConf").build().validate().is_ok());
        let err = Event::builder()
            .build()
            .validate()
            .expect_err("should be missing");
        assert!(matches!(err, crate::MatchingError::MissingField(_)));
    }

    #[test]
    fn event_round_trips_through_serde() {
        let e = Event::builder()
            .name("Glastonbury Festival 2024")
            .add_alternate_name("Glasto 2024")
            .start_date("2024-06-26T09:00:00Z")
            .end_date("2024-06-30T23:59:00Z")
            .category(EventCategory::Festival)
            .add_event_id(EventId::new(EventIdScheme::Wikidata, "Q15290").unwrap())
            .country_code_as_iso_3166_1_alpha_2("GB")
            .build();
        let json = serde_json::to_string(&e).expect("serialise");
        let back: Event = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(e, back);
    }

    #[test]
    fn alternate_names_setter_replaces_vec() {
        let e = Event::builder()
            .alternate_names(vec!["X".into(), "Y".into()])
            .build();
        assert_eq!(e.alternate_names, vec!["X".to_string(), "Y".to_string()]);
    }

    #[test]
    fn event_id_trims_value() {
        let id = EventId::new(EventIdScheme::Eventbrite, "   abc ").unwrap();
        assert_eq!(id.value, "abc");
    }

    #[test]
    fn event_id_rejects_empty() {
        assert!(EventId::new(EventIdScheme::Eventbrite, "").is_none());
        assert!(EventId::new(EventIdScheme::Eventbrite, "    ").is_none());
    }

    #[test]
    fn event_id_equality_is_scheme_scoped() {
        let g = EventId::new(EventIdScheme::Eventbrite, "X").unwrap();
        let o = EventId::new(EventIdScheme::Meetup, "X").unwrap();
        assert_ne!(g, o);
    }

    #[test]
    fn event_id_scheme_other_compares_by_string() {
        let a = EventId::new(EventIdScheme::Other("foo".into()), "1").unwrap();
        let b = EventId::new(EventIdScheme::Other("bar".into()), "1").unwrap();
        let c = EventId::new(EventIdScheme::Other("foo".into()), "1").unwrap();
        assert_ne!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn category_other_compares_by_string() {
        assert_ne!(
            EventCategory::Other("foo".into()),
            EventCategory::Other("bar".into())
        );
        assert_eq!(
            EventCategory::Other("foo".into()),
            EventCategory::Other("foo".into())
        );
    }

    #[test]
    fn event_status_roundtrips_through_serde() {
        let s = EventStatus::EventCancelled;
        let json = serde_json::to_string(&s).expect("serialise");
        let back: EventStatus = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(s, back);
    }

    #[test]
    fn attendance_mode_roundtrips_through_serde() {
        let m = EventAttendanceMode::MixedEventAttendanceMode;
        let json = serde_json::to_string(&m).expect("serialise");
        let back: EventAttendanceMode = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(m, back);
    }
}
