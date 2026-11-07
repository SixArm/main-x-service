//! Row structs: one plain struct per table this service reads whole.
//!
//! Each field is named and typed exactly as its column, so a `query_as!`
//! that selects the columns by name fills the struct, and the compiler
//! checks both against the schema (`.sqlx/`). There is no behaviour here,
//! and no hidden SQL: every statement is written out in the module that
//! runs it (`repositories.rs`, `audit.rs`, `outbox.rs`).
//!
//! Layout: a single `events` row holds the scalar event fields. Repeated
//! relational data lives in child tables: `event_identifiers`,
//! `event_locations`, `event_parties`, `event_offers`, `event_links`,
//! `event_sub_events`, and the tagged `event_text_values` (alternate
//! names, images, `same_as`, keywords, languages). The audit log and the
//! outbox have their own tables.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// One row of the `events` table (the event aggregate root).
#[derive(Clone, Debug, PartialEq)]
pub struct EventRow {
    /// Primary key (UUID).
    pub id: Uuid,
    /// Soft-delete / visibility flag.
    pub active: bool,
    /// Event title.
    pub name: String,
    /// SHA-256 (FIPS 180-4) over the assembled record's pre-image.
    ///
    /// `None` on a row written before the column existed — reported
    /// as unhashed, never as a mismatch, and never back-filled.
    pub content_hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image.
    pub content_hash_sha3: Option<String>,
    /// HMAC-SHA256 over the same pre-image — the only one of the
    /// three an adversary holding just this database cannot forge.
    pub content_mac: Option<String>,
    /// Long-form description.
    pub description: Option<String>,
    /// Short distinguishing description.
    pub disambiguating_description: Option<String>,
    /// Canonical URL.
    pub url: Option<String>,
    /// Event start instant (required).
    pub start_date: DateTime<Utc>,
    /// Event end instant, if known.
    pub end_date: Option<DateTime<Utc>>,
    /// Admission/door-open instant.
    pub door_time: Option<DateTime<Utc>>,
    /// ISO 8601 duration string.
    pub duration: Option<String>,
    /// Original start instant before any reschedule.
    pub previous_start_date: Option<DateTime<Utc>>,
    /// IANA time-zone for display (storage is UTC).
    pub time_zone: Option<String>,
    /// All-day flag.
    pub all_day: bool,
    /// Serialized `EventStatus`.
    pub event_status: String,
    /// Serialized `EventAttendanceMode`.
    pub event_attendance_mode: String,
    /// Serialized `EventType`.
    pub event_type: String,
    /// Typical audience age range.
    pub typical_age_range: Option<String>,
    /// Whether the event is free to attend.
    pub is_accessible_for_free: Option<bool>,
    /// Total maximum attendee capacity.
    pub maximum_attendee_capacity: Option<i32>,
    /// Maximum physical (in-person) capacity.
    pub maximum_physical_attendee_capacity: Option<i32>,
    /// Maximum virtual (remote) capacity.
    pub maximum_virtual_attendee_capacity: Option<i32>,
    /// Remaining (unfilled) capacity.
    pub remaining_attendee_capacity: Option<i32>,
    /// Parent event id (schema.org `superEvent`).
    pub super_event_id: Option<Uuid>,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Row last-update timestamp.
    pub updated_at: DateTime<Utc>,
    /// User who created the row.
    pub created_by: Option<String>,
    /// User who last updated the row.
    pub updated_by: Option<String>,
    /// Soft-delete timestamp.
    pub deleted_at: Option<DateTime<Utc>>,
    /// User who soft-deleted the row.
    pub deleted_by: Option<String>,
}

/// One external identifier belonging to an event (`event_identifiers`).
#[derive(Clone, Debug, PartialEq)]
pub struct IdentifierRow {
    /// Primary key (UUID).
    pub id: Uuid,
    /// Owning event id.
    pub event_id: Uuid,
    /// Optional identifier use/category.
    pub use_type: Option<String>,
    /// Serialized `IdentifierType`.
    pub identifier_type: String,
    /// Issuing system URI/name.
    pub system: String,
    /// The identifier value.
    pub value: String,
    /// Optional assigning authority.
    pub assigner: Option<String>,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Row last-update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// One location row for an event (`event_locations`). The `kind` column
/// discriminates the `Location` variant; only that variant's columns are
/// populated, and `position` preserves ordering.
#[derive(Clone, Debug, PartialEq)]
pub struct LocationRow {
    /// Primary key (UUID).
    pub id: Uuid,
    /// Owning event id.
    pub event_id: Uuid,
    /// Ordering position within the event's location list.
    pub position: i32,
    /// "place" | "`postal_address`" | "virtual" | "text"
    pub kind: String,
    /// For "place": optional external place-service id.
    pub place_id: Option<Uuid>,
    /// For "place": display name. For "virtual": optional name.
    /// For "text": the text value.
    pub name: Option<String>,
    /// Address line 1.
    pub line1: Option<String>,
    /// Address line 2.
    pub line2: Option<String>,
    /// City / locality.
    pub city: Option<String>,
    /// State / region.
    pub state: Option<String>,
    /// Postal code.
    pub postal_code: Option<String>,
    /// Country.
    pub country: Option<String>,
    /// Latitude (for "place"). `NUMERIC` — exact decimal degrees.
    pub latitude_as_decimal_degrees: Option<BigDecimal>,
    /// Longitude (for "place"). `NUMERIC` — exact decimal degrees.
    pub longitude_as_decimal_degrees: Option<BigDecimal>,
    /// URL (for "place" or "virtual").
    pub url: Option<String>,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Row last-update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// One party row attached to an event in a particular role
/// (`event_parties`: organizers, performers, attendees, sponsors, funders,
/// contributors).
#[derive(Clone, Debug, PartialEq)]
pub struct PartyRow {
    /// Primary key (UUID).
    pub id: Uuid,
    /// Owning event id.
    pub event_id: Uuid,
    /// Ordering position within the role list.
    pub position: i32,
    /// "organizer" | "performer" | "attendee" | "sponsor" |
    /// "funder" | "contributor"
    pub role: String,
    /// "person" | "organization"
    pub party_kind: String,
    /// Optional external person/org-service id.
    pub party_id: Option<Uuid>,
    /// Party display name.
    pub name: String,
    /// Optional contact email.
    pub email: Option<String>,
    /// Optional URL.
    pub url: Option<String>,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Row last-update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// One offer row (`event_offers`: price, currency, availability, validity
/// window).
#[derive(Clone, Debug, PartialEq)]
pub struct OfferRow {
    /// Primary key (UUID).
    pub id: Uuid,
    /// Owning event id.
    pub event_id: Uuid,
    /// Ordering position within the offer list.
    pub position: i32,
    /// Offer name/label.
    pub name: Option<String>,
    /// Price as a fixed-precision decimal (`NUMERIC(12, 4)`).
    pub price: Option<BigDecimal>,
    /// ISO 4217 currency code.
    pub price_currency: Option<String>,
    /// Offer URL.
    pub url: Option<String>,
    /// Serialized `OfferAvailability`.
    pub availability: Option<String>,
    /// Offer validity start.
    pub valid_from: Option<DateTime<Utc>>,
    /// Offer validity end.
    pub valid_through: Option<DateTime<Utc>>,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Row last-update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// One typed link from an event to another event (`event_links`).
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRow {
    /// Primary key (UUID).
    pub id: Uuid,
    /// Owning (source) event id.
    pub event_id: Uuid,
    /// Target event id.
    pub other_event_id: Uuid,
    /// Serialized `LinkType` (Replaces / `ReplacedBy` / Refer / Seealso).
    pub link_type: String,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
    /// User who created the link.
    pub created_by: Option<String>,
}

/// One ordered sub-event membership row (`event_sub_events`, the
/// schema.org `subEvent` list).
#[derive(Clone, Debug, PartialEq)]
pub struct SubEventRow {
    /// Primary key (UUID).
    pub id: Uuid,
    /// Parent event id.
    pub event_id: Uuid,
    /// Child (sub) event id.
    pub sub_event_id: Uuid,
    /// Ordering position within the sub-event list.
    pub position: i32,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// One `(field, value)` row for an event string list
/// (`event_text_values`: `alternate_name` | image | `same_as` | keyword |
/// `in_language`).
#[derive(Clone, Debug, PartialEq)]
pub struct TextValueRow {
    /// Primary key.
    pub id: Uuid,
    /// FK to the owning event.
    pub event_id: Uuid,
    /// Which list this value belongs to.
    pub field: String,
    /// The value.
    pub value: String,
    /// Ordinal position within the list.
    pub position: i32,
}

/// One audit-log entry recording a who/what/when change (`audit_log`).
///
/// The audit endpoints serialize this row as it is, so its JSON shape is
/// part of the API. `timestamp` is serialized through the `time` crate's
/// form, exactly as it was when this row was an ORM model typed
/// `time::OffsetDateTime`, so the wire is unchanged.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AuditLogRow {
    /// Primary key (UUID).
    pub id: Uuid,
    /// When the action occurred.
    #[serde(serialize_with = "serialize_as_time")]
    pub timestamp: DateTime<Utc>,
    /// User who performed the action.
    pub user_id: Option<String>,
    /// Action name (create / update / delete / merge / …).
    pub action: String,
    /// Type of entity affected (e.g. "event").
    pub entity_type: String,
    /// Id of the entity affected.
    pub entity_id: Uuid,
    /// JSON snapshot of values before the change.
    pub old_values: Option<serde_json::Value>,
    /// JSON snapshot of values after the change.
    pub new_values: Option<serde_json::Value>,
    /// Originating IP address.
    pub ip_address: Option<String>,
    /// Originating user-agent string.
    pub user_agent: Option<String>,
    /// SHA-256 (FIPS 180-4) over this audit row's pre-image.
    ///
    /// Unkeyed, so anyone holding the database can recompute it — what it
    /// catches is careless or unaware modification. Written
    /// unconditionally, unlike the MAC, which needs a key: with no key
    /// configured these two digests are the row's only integrity.
    pub hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
    /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
    /// against one design family does not transfer.
    pub hash_sha3: Option<String>,
    /// HMAC-SHA256 over this audit row's pre-image.
    ///
    /// Detects a row whose content was altered. It does **not**
    /// detect a row deleted wholesale — that needs the hash chain
    /// this service does not yet have (see `crate::compliance`).
    pub mac: Option<String>,
}

/// Serialize a timestamp as `time::OffsetDateTime` does (see
/// [`AuditLogRow`]): the same instant, in UTC.
fn serialize_as_time<S: serde::Serializer>(
    ts: &DateTime<Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    super::convert::ts_to_offset(*ts).serialize(serializer)
}

/// One `event_outbox` row — the transactional-outbox hand-off buffer for
/// the durable event bus (Phase 2; `agents/share/event-bus.md` §3). One
/// row is written **in the same transaction** as the entity mutation, so
/// a committed change always has its event and vice versa; the relay
/// (`crate::relay`) drains unpublished rows and stamps `published_at`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboxRow {
    /// `BIGSERIAL` pk; also the global relay order (`ORDER BY id`).
    pub id: i64,
    /// Envelope id — the consumer dedup key (`UNIQUE`).
    pub event_id: Uuid,
    /// The entity name (`event`).
    pub entity: String,
    /// The record pid — the bus partition key.
    pub entity_pid: Uuid,
    /// The change kind: `created` / `updated` / `deleted` / `merged`.
    pub kind: String,
    /// When the change occurred (stamped at enqueue).
    pub occurred_at: DateTime<Utc>,
    /// The acting user pid, or `None` when unauthenticated.
    pub actor: Option<String>,
    /// The envelope schema version.
    pub schema_version: i32,
    /// The full canonical envelope as JSONB.
    pub payload: serde_json::Value,
    /// `None` until the relay ships the row to the bus.
    pub published_at: Option<DateTime<Utc>>,
}
