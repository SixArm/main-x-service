//! SeaORM database entities for the event service.
//!
//! Layout: a single `events` row holds scalar event fields plus
//! short JSONB arrays for `alternate_names`, `image`, `same_as`,
//! `keywords`, and `in_language`. Repeated relational data lives
//! in child tables: `event_identifiers`, `event_locations`,
//! `event_parties`, `event_offers`, `event_links`, and
//! `event_sub_events`. Organizations and audit log retain their own
//! tables.

use serde::{Deserialize, Serialize};

// ============================================================================
// events
// ============================================================================

/// `SeaORM` entity for the `events` table (the event aggregate root).
pub mod events {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One row of the `events` table: scalar event fields plus JSONB
    /// arrays for short multi-valued attributes.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "events")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
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
        pub start_date: TimeDateTimeWithTimeZone,
        /// Event end instant, if known.
        pub end_date: Option<TimeDateTimeWithTimeZone>,
        /// Admission/door-open instant.
        pub door_time: Option<TimeDateTimeWithTimeZone>,
        /// ISO 8601 duration string.
        pub duration: Option<String>,
        /// Original start instant before any reschedule.
        pub previous_start_date: Option<TimeDateTimeWithTimeZone>,
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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// User who created the row.
        pub created_by: Option<String>,
        /// User who last updated the row.
        pub updated_by: Option<String>,
        /// Soft-delete timestamp.
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
        /// User who soft-deleted the row.
        pub deleted_by: Option<String>,
    }

    /// Relations from `events` to its child tables.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `has_many` `event_identifiers`.
        #[sea_orm(has_many = "super::event_identifiers::Entity")]
        Identifiers,
        /// `has_many` `event_locations`.
        #[sea_orm(has_many = "super::event_locations::Entity")]
        Locations,
        /// `has_many` `event_parties`.
        #[sea_orm(has_many = "super::event_parties::Entity")]
        Parties,
        /// `has_many` `event_offers`.
        #[sea_orm(has_many = "super::event_offers::Entity")]
        Offers,
        /// `has_many` `event_links`.
        #[sea_orm(has_many = "super::event_links::Entity")]
        Links,
        /// `has_many` `event_sub_events`.
        #[sea_orm(has_many = "super::event_sub_events::Entity")]
        SubEvents,
    }

    impl Related<super::event_identifiers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Identifiers.def()
        }
    }
    impl Related<super::event_locations::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Locations.def()
        }
    }
    impl Related<super::event_parties::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Parties.def()
        }
    }
    impl Related<super::event_offers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Offers.def()
        }
    }
    impl Related<super::event_links::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Links.def()
        }
    }
    impl Related<super::event_sub_events::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::SubEvents.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// event_identifiers
// ============================================================================

/// `SeaORM` entity for the `event_identifiers` child table.
pub mod event_identifiers {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One external identifier belonging to an event.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_identifiers")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Relation back to the owning event.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent event.
        #[sea_orm(
            belongs_to = "super::events::Entity",
            from = "Column::EventId",
            to = "super::events::Column::Id"
        )]
        Event,
    }

    impl Related<super::events::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Event.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// event_locations
//
// `kind` discriminates the variant; only the columns relevant to that
// variant are populated. `position` preserves ordering.
// ============================================================================

/// `SeaORM` entity for the `event_locations` child table. The `kind`
/// column discriminates the `Location` variant; only that variant's
/// columns are populated, and `position` preserves ordering.
pub mod event_locations {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One location row for an event (place / postal / virtual / text).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_locations")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
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
        pub latitude_as_decimal_degrees: Option<bigdecimal::BigDecimal>,
        /// Longitude (for "place"). `NUMERIC` — exact decimal degrees.
        pub longitude_as_decimal_degrees: Option<bigdecimal::BigDecimal>,
        /// URL (for "place" or "virtual").
        pub url: Option<String>,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Relation back to the owning event.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent event.
        #[sea_orm(
            belongs_to = "super::events::Entity",
            from = "Column::EventId",
            to = "super::events::Column::Id"
        )]
        Event,
    }

    impl Related<super::events::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Event.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// event_parties (organizer / performer / attendee / sponsor / ...)
// ============================================================================

/// `SeaORM` entity for the `event_parties` child table (organizers,
/// performers, attendees, sponsors, funders, contributors).
pub mod event_parties {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One party row attached to an event in a particular role.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_parties")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Relation back to the owning event.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent event.
        #[sea_orm(
            belongs_to = "super::events::Entity",
            from = "Column::EventId",
            to = "super::events::Column::Id"
        )]
        Event,
    }

    impl Related<super::events::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Event.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// event_offers
// ============================================================================

/// `SeaORM` entity for the `event_offers` child table (ticket/pricing tiers).
pub mod event_offers {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One offer row (price, currency, availability, validity window).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_offers")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning event id.
        pub event_id: Uuid,
        /// Ordering position within the offer list.
        pub position: i32,
        /// Offer name/label.
        pub name: Option<String>,
        /// Price as a fixed-precision decimal.
        #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
        pub price: Option<bigdecimal::BigDecimal>,
        /// ISO 4217 currency code.
        pub price_currency: Option<String>,
        /// Offer URL.
        pub url: Option<String>,
        /// Serialized `OfferAvailability`.
        pub availability: Option<String>,
        /// Offer validity start.
        pub valid_from: Option<TimeDateTimeWithTimeZone>,
        /// Offer validity end.
        pub valid_through: Option<TimeDateTimeWithTimeZone>,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Relation back to the owning event.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent event.
        #[sea_orm(
            belongs_to = "super::events::Entity",
            from = "Column::EventId",
            to = "super::events::Column::Id"
        )]
        Event,
    }

    impl Related<super::events::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Event.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// event_links
// ============================================================================

/// `SeaORM` entity for the `event_links` child table (cross-event links).
pub mod event_links {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One typed link from an event to another event.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_links")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning (source) event id.
        pub event_id: Uuid,
        /// Target event id.
        pub other_event_id: Uuid,
        /// Serialized `LinkType` (Replaces / `ReplacedBy` / Refer / Seealso).
        pub link_type: String,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// User who created the link.
        pub created_by: Option<String>,
    }

    /// Relation back to the owning event.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent event.
        #[sea_orm(
            belongs_to = "super::events::Entity",
            from = "Column::EventId",
            to = "super::events::Column::Id"
        )]
        Event,
    }

    impl Related<super::events::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Event.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// event_sub_events (the schema.org/subEvent list)
// ============================================================================

/// `SeaORM` entity for the `event_sub_events` child table (schema.org
/// `subEvent` ordering list).
pub mod event_sub_events {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One ordered sub-event membership row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_sub_events")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Parent event id.
        pub event_id: Uuid,
        /// Child (sub) event id.
        pub sub_event_id: Uuid,
        /// Ordering position within the sub-event list.
        pub position: i32,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
    }

    /// Relation back to the owning (parent) event.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent event.
        #[sea_orm(
            belongs_to = "super::events::Entity",
            from = "Column::EventId",
            to = "super::events::Column::Id"
        )]
        Event,
    }

    impl Related<super::events::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Event.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// audit_log (unchanged)
// ============================================================================

/// `SeaORM` entity for the `audit_log` table (HIPAA-style audit trail).
pub mod audit_log {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One audit-log entry recording a who/what/when change.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// When the action occurred.
        pub timestamp: TimeDateTimeWithTimeZone,
        /// User who performed the action.
        pub user_id: Option<String>,
        /// Action name (create / update / delete / merge / …).
        pub action: String,
        /// Type of entity affected (e.g. "event").
        pub entity_type: String,
        /// Id of the entity affected.
        pub entity_id: Uuid,
        /// JSON snapshot of values before the change.
        pub old_values: Option<Json>,
        /// JSON snapshot of values after the change.
        pub new_values: Option<Json>,
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

    /// `audit_log` has no relations.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── event_text_values ─────────────────────

/// Tagged table for the Event's parallel string-list properties
/// (`alternate_name` | image | `same_as` | keyword | `in_language`).
pub mod event_text_values {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One `(field, value)` row for an event string list.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_text_values")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
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

    /// `belongs_to` the parent event.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `events`.
        #[sea_orm(
            belongs_to = "super::events::Entity",
            from = "Column::EventId",
            to = "super::events::Column::Id"
        )]
        Event,
    }
    impl Related<super::events::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Event.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// event_outbox
// ============================================================================

/// `SeaORM` entity for the `event_outbox` table — the transactional-outbox
/// hand-off buffer for the durable event bus (Phase 2;
/// `agents/share/event-bus.md` §3). One row is written **in the same
/// transaction** as the entity mutation, so a committed change always has
/// its event and vice versa; a relay worker (Phase 3, roadmap) drains
/// unpublished rows to Fluvio and stamps `published_at`.
pub mod event_outbox {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One persisted outbox row: a canonical envelope awaiting relay to
    /// the durable bus.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_outbox")]
    pub struct Model {
        /// Auto-increment pk; also the global relay order (`ORDER BY id`).
        #[sea_orm(primary_key)]
        pub id: i64,
        /// Envelope id — the consumer dedup key.
        #[sea_orm(unique)]
        pub event_id: Uuid,
        /// The entity name (`event`).
        pub entity: String,
        /// The record pid — the bus partition key.
        pub entity_pid: Uuid,
        /// The change kind: `created` / `updated` / `deleted` / `merged`.
        pub kind: String,
        /// When the change occurred (stamped at enqueue).
        pub occurred_at: TimeDateTimeWithTimeZone,
        /// The acting user pid, or `None` when unauthenticated.
        pub actor: Option<String>,
        /// The envelope schema version.
        pub schema_version: i32,
        /// The full canonical envelope as JSONB.
        pub payload: Json,
        /// `None` until the relay ships the row to the bus.
        pub published_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// `SeaORM` relations for the outbox entity (none defined).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
