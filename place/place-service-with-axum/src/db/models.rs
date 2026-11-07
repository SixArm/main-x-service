//! Row structs: one plain struct per table this service reads whole.
//!
//! Each field is named and typed exactly as its column, so a `query_as!`
//! that selects the columns by name fills the struct, and the compiler
//! checks both against the schema (`.sqlx/`). There is no behaviour here,
//! and no hidden SQL: every statement is written out in the module that
//! runs it (`src/db/mod.rs`, `audit.rs`, `outbox.rs`, `review_queue.rs`).
//!
//! The schema is fully normalized: the 0..1 nested objects
//! (`PostalAddress`, `GeoCoordinates`) are flattened onto the `places`
//! row; the repeating collections (keywords, identifiers,
//! `amenity_features`, `opening_hours`) live in child tables joined by
//! `place_id`, read straight into the domain types. JSONB is used only for
//! opaque snapshots (audit old/new values, merge `transferred_data`, the
//! outbox envelope).

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A row in `places`. Soft-delete is via `is_deleted`/`deleted_at`.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaceRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Primary name.
    pub name: String,
    /// Alternate name / alias.
    pub alternate_name: Option<String>,
    /// Free-text description.
    pub description: Option<String>,
    /// `PlaceType` enum tag (or `other`).
    pub place_type: Option<String>,
    /// Free-text label for `PlaceType::Other(_)`.
    pub place_type_custom: Option<String>,
    /// `PostalAddress.street_address` (flattened).
    pub address_street_address: Option<String>,
    /// `PostalAddress.address_locality` (flattened).
    pub address_locality: Option<String>,
    /// `PostalAddress.address_region` (flattened).
    pub address_region: Option<String>,
    /// `PostalAddress.address_country` (flattened).
    pub address_country: Option<String>,
    /// `PostalAddress.postal_code` (flattened).
    pub address_postal_code: Option<String>,
    /// `GeoCoordinates.latitude_as_decimal_degrees` (flattened).
    pub geo_latitude_as_decimal_degrees: Option<BigDecimal>,
    /// `GeoCoordinates.longitude_as_decimal_degrees` (flattened).
    pub geo_longitude_as_decimal_degrees: Option<BigDecimal>,
    /// `GeoCoordinates.elevation_as_decimal_metres` (flattened).
    pub geo_elevation_as_decimal_metres: Option<BigDecimal>,
    /// Telephone number.
    pub telephone: Option<String>,
    /// Fax number.
    pub fax_number: Option<String>,
    /// Website URL.
    pub url: Option<String>,
    /// GS1 Global Location Number.
    pub global_location_number: Option<String>,
    /// Operator-defined branch / site code.
    pub branch_code: Option<String>,
    /// Parent place id (containedInPlace).
    pub contained_in_place: Option<Uuid>,
    /// Whether the place is free to access.
    pub is_accessible_for_free: Option<bool>,
    /// Whether the place is open to the public.
    pub public_access: Option<bool>,
    /// Whether smoking is permitted.
    pub smoking_allowed: Option<bool>,
    /// Maximum capacity.
    pub maximum_attendee_capacity: Option<i32>,
    /// Soft-delete flag.
    pub is_deleted: bool,
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
    /// Soft-delete timestamp.
    pub deleted_at: Option<DateTime<Utc>>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last-update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// One `audit_log` row (old/new values are opaque snapshots).
#[derive(Clone, Debug, PartialEq)]
pub struct AuditLogRow {
    /// Primary key.
    pub id: Uuid,
    /// Entity type the action targeted.
    pub entity_type: String,
    /// Id of the affected entity.
    pub entity_id: Uuid,
    /// Action performed.
    pub action: String,
    /// Acting user id.
    pub user_id: Option<String>,
    /// Request IP address.
    pub user_ip_address: Option<String>,
    /// Request user-agent.
    pub user_agent: Option<String>,
    /// Pre-change snapshot (opaque).
    pub old_values: Option<serde_json::Value>,
    /// Post-change snapshot (opaque).
    pub new_values: Option<serde_json::Value>,
    /// When the action occurred.
    pub created_at: DateTime<Utc>,
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
    /// The entity name (`place`).
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
