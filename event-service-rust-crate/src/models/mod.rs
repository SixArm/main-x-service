//! Domain models for the Event Service.
//!
//! The central model is [`Event`], aligned with
//! <https://schema.org/Event>. An event is a time-bounded occurrence
//! (conference, performance, appointment, shift, encounter,
//! delivery, sale, screening, …) with a location, parties, and
//! optional offers.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub mod consent;
pub mod event;
pub mod identifier;
pub mod merge;
pub mod organization;
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

/// A postal address. Mirrors schema.org/PostalAddress.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Address {
    pub use_type: Option<AddressUse>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AddressUse {
    Home,
    Work,
    Temp,
    Old,
    Billing,
}

// ===========================================================================
// ContactPoint
// ===========================================================================

/// A way to contact a party (phone, email, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct ContactPoint {
    pub system: ContactPointSystem,
    pub value: String,
    pub use_type: Option<ContactPointUse>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointSystem {
    Phone,
    Fax,
    Email,
    Pager,
    Url,
    Sms,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContactPointUse {
    Home,
    Work,
    Temp,
    Old,
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
    Place(Place),
    PostalAddress(Address),
    Virtual(VirtualLocation),
    Text { value: String },
}

/// A physical place. Mirrors schema.org/Place. Optionally references
/// a record in a sibling place service via [`Place::id`].
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Place {
    /// External place-service ID, if known.
    pub id: Option<Uuid>,
    pub name: String,
    pub address: Option<Address>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub url: Option<String>,
}

/// A virtual location (e.g., a video-conference URL).
/// Mirrors schema.org/VirtualLocation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct VirtualLocation {
    pub name: Option<String>,
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
    pub kind: PartyKind,
    pub id: Option<Uuid>,
    pub name: String,
    pub email: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PartyKind {
    Person,
    Organization,
}

// ===========================================================================
// Reference (for about / workFeatured / workPerformed)
// ===========================================================================

/// A typed pointer to another resource (Thing, CreativeWork, …).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct Reference {
    pub id: Option<Uuid>,
    pub name: String,
    pub url: Option<String>,
    /// Optional schema.org type (e.g., "CreativeWork", "Thing").
    pub kind: Option<String>,
}

// ===========================================================================
// Offer (schema.org/Offer — pricing / tickets)
// ===========================================================================

/// A pricing offer for an event (e.g., a ticket tier).
/// Mirrors a minimal schema.org/Offer.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Offer {
    pub name: Option<String>,
    /// Decimal price as string for precision.
    pub price: Option<String>,
    /// ISO 4217 currency code (e.g., "USD", "EUR").
    pub price_currency: Option<String>,
    pub url: Option<String>,
    pub availability: Option<OfferAvailability>,
    pub valid_from: Option<chrono::DateTime<chrono::Utc>>,
    pub valid_through: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfferAvailability {
    InStock,
    SoldOut,
    PreOrder,
    OutOfStock,
    Discontinued,
}

// ===========================================================================
// Event status / attendance mode / type
// ===========================================================================

/// Lifecycle status of an event. Mirrors schema.org/EventStatusType.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    #[default]
    Scheduled,
    Cancelled,
    MovedOnline,
    Postponed,
    Rescheduled,
    Completed,
}

/// How attendees join an event. Mirrors
/// schema.org/EventAttendanceModeEnumeration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventAttendanceMode {
    #[default]
    Offline,
    Online,
    Mixed,
}

/// Discriminates event subtypes (mirrors schema.org Event subtypes
/// like ConferenceEvent / SportsEvent / EducationEvent, plus operational
/// subtypes such as Appointment / Encounter / Shift / Incident).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    #[default]
    Generic,
    Appointment,
    Business,
    Childrens,
    Comedy,
    Conference,
    Course,
    Dance,
    Delivery,
    Education,
    Encounter,
    Exhibition,
    Festival,
    Food,
    Hackathon,
    Incident,
    Literary,
    Music,
    PerformingArts,
    Publication,
    Sale,
    Screening,
    Series,
    Session,
    Shift,
    Social,
    Sports,
    Theater,
    VisualArts,
}
