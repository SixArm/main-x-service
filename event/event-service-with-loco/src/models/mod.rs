//! Domain models for the Event Service.
//!
//! The central model is [`Event`](crate::models::event::Event), aligned
//! with <https://schema.org/Event>. An event is a time-bounded
//! occurrence (conference, performance, appointment, shift, encounter,
//! delivery, sale, screening, …) with a location, parties, and
//! optional offers.
//!
//! This module is the crate's value-object library. It defines:
//!
//! - The supporting value types in this file
//!   ([`Address`](crate::models::Address),
//!   [`ContactPoint`](crate::models::ContactPoint),
//!   [`Location`](crate::models::Location) and its members
//!   [`Place`](crate::models::Place) /
//!   [`VirtualLocation`](crate::models::VirtualLocation),
//!   [`Party`](crate::models::Party),
//!   [`Reference`](crate::models::Reference),
//!   [`Offer`](crate::models::Offer), and the classification enums
//!   [`EventStatus`](crate::models::EventStatus) /
//!   [`EventAttendanceMode`](crate::models::EventAttendanceMode) /
//!   [`EventType`](crate::models::EventType)).
//! - The [`Event`](crate::models::event::Event) aggregate itself
//!   (submodule [`event`](crate::models::event)).
//! - External [`Identifier`](crate::models::identifier::Identifier)s
//!   (submodule [`identifier`](crate::models::identifier)).
//! - Operational records:
//!   [`Consent`](crate::models::consent::Consent),
//!   [`Organization`](crate::models::organization::Organization), merge
//!   records ([`MergeRecord`](crate::models::merge::MergeRecord) et
//!   al.), and the dedup review queue
//!   ([`ReviewQueueItem`](crate::models::review_queue::ReviewQueueItem)
//!   et al.).
//!
//! These types carry no business logic beyond simple constructors; the
//! matching, validation, and privacy layers operate *on* them.
//!
//! # Examples
//!
//! ```
//! use event_service::models::{Location, VirtualLocation};
//!
//! // A `Location` is a tagged union; here, a livestream URL.
//! let loc = Location::Virtual(VirtualLocation {
//!     name: Some("Livestream".into()),
//!     url: "https://example.test/stream".into(),
//! });
//! // The `kind` tag is what serde uses to discriminate on the wire.
//! let json = serde_json::to_string(&loc).unwrap();
//! assert!(json.contains("\"kind\":\"virtual\""));
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Consent records (GDPR / media-release / data-sharing).
pub mod consent;
/// The [`Event`] aggregate root.
pub mod event;
/// External [`Identifier`]s and their classification.
pub mod identifier;
/// Merge request / response / history records.
pub mod merge;
/// The [`Organization`] record (richer than a [`Party`] reference).
pub mod organization;
/// Deduplication review-queue and batch-dedup request / response.
pub mod review_queue;

pub use consent::{Consent, ConsentStatus, ConsentType};
pub use event::{Event, EventLink, LinkType};
pub use identifier::{Identifier, IdentifierType, IdentifierUse};
pub use merge::{MergeRecord, MergeRequest, MergeResponse, MergeStatus};
pub use organization::Organization;
pub use review_queue::{
    BatchDeduplicationRequest, BatchDeduplicationResponse, ReviewQueueItem, ReviewStatus,
};

// ===========================================================================
// Address (PostalAddress)
// ===========================================================================

/// A postal address. Mirrors [schema.org/PostalAddress](https://schema.org/PostalAddress).
///
/// Every field is optional so that partial addresses (e.g. just a city
/// and country) can be represented; the validation layer decides which
/// combinations are acceptable for a given use.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Address {
    /// What this address is used for (home / work / billing / …).
    pub use_type: Option<AddressUse>,
    /// First street-address line (number + street).
    pub line1: Option<String>,
    /// Second street-address line (suite, unit, floor).
    pub line2: Option<String>,
    /// City / locality.
    pub city: Option<String>,
    /// State / province / region.
    pub state: Option<String>,
    /// Postal / ZIP code.
    pub postal_code: Option<String>,
    /// Country (name or ISO code, per caller convention).
    pub country: Option<String>,
}

/// Intended use of an [`Address`]. Mirrors the FHIR address-use value set.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AddressUse {
    /// A residential / home address.
    Home,
    /// A business / work address.
    Work,
    /// A temporary address.
    Temp,
    /// A former address no longer in use.
    Old,
    /// An address used specifically for billing.
    Billing,
}

// ===========================================================================
// ContactPoint
// ===========================================================================

/// A way to contact a party (phone, email, etc.). Mirrors
/// [schema.org/ContactPoint](https://schema.org/ContactPoint) /
/// FHIR `ContactPoint`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct ContactPoint {
    /// The channel kind (phone / email / url / …).
    pub system: ContactPointSystem,
    /// The contact value (a phone number, email address, URL, …),
    /// stored verbatim as supplied.
    pub value: String,
    /// What context this contact applies to (home / work / mobile / …).
    pub use_type: Option<ContactPointUse>,
}

/// The communication channel of a [`ContactPoint`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointSystem {
    /// A telephone number.
    Phone,
    /// A fax number.
    Fax,
    /// An email address.
    Email,
    /// A pager number.
    Pager,
    /// A web URL.
    Url,
    /// An SMS-capable number.
    Sms,
    /// Any other channel.
    Other,
}

/// The context in which a [`ContactPoint`] is used.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointUse {
    /// A home contact.
    Home,
    /// A work contact.
    Work,
    /// A temporary contact.
    Temp,
    /// A former contact no longer in use.
    Old,
    /// A mobile contact.
    Mobile,
}

// ===========================================================================
// Location (schema.org Event.location = Place | PostalAddress | VirtualLocation | Text)
// ===========================================================================

/// Where an event takes place.
///
/// Mirrors the schema.org/Event `location` property, which can be a
/// physical place, a postal address, a virtual location (URL), or
/// free text.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    /// A physical venue with a name and optional geo coordinates.
    Place(Place),
    /// An inline postal address (no separate place record).
    PostalAddress(Address),
    /// A virtual location, e.g. a livestream or video-conference URL.
    Virtual(VirtualLocation),
    /// A free-text fallback (e.g. "TBA") with no structured data.
    Text {
        /// The raw location text.
        value: String,
    },
}

/// A physical place. Mirrors [schema.org/Place](https://schema.org/Place).
/// Optionally references a record in a sibling place service via
/// [`Place::id`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Place {
    /// External place-service ID, if known. When two places share this
    /// ID, the matcher treats them as the same venue (short-circuit).
    pub id: Option<Uuid>,
    /// Venue name (required).
    pub name: String,
    /// Structured postal address of the venue.
    pub address: Option<Address>,
    /// Latitude in decimal degrees (`-90.0..=90.0`).
    pub latitude: Option<f64>,
    /// Longitude in decimal degrees (`-180.0..=180.0`).
    pub longitude: Option<f64>,
    /// Canonical URL for the venue.
    pub url: Option<String>,
}

/// A virtual location (e.g., a video-conference URL).
/// Mirrors [schema.org/VirtualLocation](https://schema.org/VirtualLocation).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct VirtualLocation {
    /// Optional display name (e.g. "Main Zoom Room").
    pub name: Option<String>,
    /// The join URL (required); used for `Virtual ↔ Virtual` matching.
    pub url: String,
}

// ===========================================================================
// Party (schema.org Person | Organization for organizer/performer/attendee/...)
// ===========================================================================

/// A person or organization associated with an event.
///
/// Used for organizers, performers, attendees, sponsors, funders, etc.
/// `id`, when present, references a record in a sibling
/// person/worker/organization service.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Party {
    /// Whether this party is a person or an organization. Parties of
    /// different kinds never match (the matcher scores them `0.0`).
    pub kind: PartyKind,
    /// External person/worker/organization-service ID, if known. A
    /// shared ID short-circuits party matching to `1.0`.
    pub id: Option<Uuid>,
    /// Display name (required).
    pub name: String,
    /// Contact email; masked in privacy views and used as a
    /// deterministic match signal.
    pub email: Option<String>,
    /// Canonical URL for the party.
    pub url: Option<String>,
}

/// Discriminates a [`Party`] between a natural person and an organization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PartyKind {
    /// A natural person.
    Person,
    /// An organization (company, troupe, clinic, …).
    Organization,
}

// ===========================================================================
// Reference (for about / workFeatured / workPerformed)
// ===========================================================================

/// A typed pointer to another resource (Thing, `CreativeWork`, …).
///
/// Used for the `about` (subject matter) and `works` (featured /
/// performed works) properties of an [`Event`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Reference {
    /// External resource ID, if known. A shared ID makes references
    /// match deterministically.
    pub id: Option<Uuid>,
    /// Display name of the referenced resource.
    pub name: String,
    /// Canonical URL of the referenced resource.
    pub url: Option<String>,
    /// Optional schema.org type (e.g., "`CreativeWork`", "Thing").
    pub kind: Option<String>,
}

// ===========================================================================
// Offer (schema.org/Offer — pricing / tickets)
// ===========================================================================

/// A pricing offer for an event (e.g., a ticket tier).
/// Mirrors a minimal [schema.org/Offer](https://schema.org/Offer).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Offer {
    /// Tier / offer name (e.g. "General Admission", "VIP").
    pub name: Option<String>,
    /// Decimal price as string for precision (avoids float rounding).
    pub price: Option<String>,
    /// ISO 4217 currency code (e.g., "USD", "EUR").
    pub price_currency: Option<String>,
    /// URL where the offer can be purchased.
    pub url: Option<String>,
    /// Stock / availability state of this offer.
    pub availability: Option<OfferAvailability>,
    /// When the offer becomes valid (sales open).
    pub valid_from: Option<DateTime<Utc>>,
    /// When the offer expires (sales close); must be `>= valid_from`.
    pub valid_through: Option<DateTime<Utc>>,
}

/// Availability state of an [`Offer`]. Mirrors
/// [schema.org/ItemAvailability](https://schema.org/ItemAvailability).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfferAvailability {
    /// In stock and purchasable now.
    InStock,
    /// Sold out (no more available).
    SoldOut,
    /// Available for pre-order before general sale.
    PreOrder,
    /// Out of stock (may return).
    OutOfStock,
    /// Permanently discontinued.
    Discontinued,
}

// ===========================================================================
// Event status / attendance mode / type
// ===========================================================================

/// Lifecycle status of an event. Mirrors schema.org/EventStatusType.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    /// Going ahead as planned (the default).
    #[default]
    Scheduled,
    /// Called off entirely.
    Cancelled,
    /// Converted from in-person to online.
    MovedOnline,
    /// Delayed to an unspecified later date.
    Postponed,
    /// Moved to a specific new date (see `previous_start_date`).
    Rescheduled,
    /// Finished.
    Completed,
}

/// How attendees join an event. Mirrors
/// schema.org/EventAttendanceModeEnumeration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventAttendanceMode {
    /// In-person only (the default). Expects a physical location.
    #[default]
    Offline,
    /// Online only. Expects at least one [`Location::Virtual`].
    Online,
    /// Both in-person and online. Expects at least one physical and
    /// one virtual location.
    Mixed,
}

/// Discriminates event subtypes (mirrors schema.org Event subtypes
/// like `ConferenceEvent` / `SportsEvent` / `EducationEvent`, plus operational
/// subtypes such as Appointment / Encounter / Shift / Incident).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Unspecified / catch-all event (the default).
    #[default]
    Generic,
    /// A scheduled appointment (booking, reservation, visit).
    Appointment,
    /// A business event (meeting, networking, …).
    Business,
    /// A children's event.
    Childrens,
    /// A comedy event.
    Comedy,
    /// A conference.
    Conference,
    /// A course session (see also the sibling course service).
    Course,
    /// A dance event.
    Dance,
    /// A delivery (operational subtype).
    Delivery,
    /// An educational event.
    Education,
    /// A clinical encounter (operational subtype).
    Encounter,
    /// An exhibition.
    Exhibition,
    /// A festival.
    Festival,
    /// A food event.
    Food,
    /// A hackathon.
    Hackathon,
    /// An incident (operational subtype).
    Incident,
    /// A literary event.
    Literary,
    /// A music event.
    Music,
    /// A performing-arts event.
    PerformingArts,
    /// A publication event.
    Publication,
    /// A sale (operational subtype).
    Sale,
    /// A screening (film / media).
    Screening,
    /// A series (umbrella over several sub-events).
    Series,
    /// A session (operational subtype).
    Session,
    /// A work shift (operational subtype).
    Shift,
    /// A social event.
    Social,
    /// A sports event.
    Sports,
    /// A theater event.
    Theater,
    /// A visual-arts event.
    VisualArts,
}
