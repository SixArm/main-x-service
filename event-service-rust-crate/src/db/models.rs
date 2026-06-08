//! SeaORM database entities for the event service.
//!
//! Layout: a single `events` row holds scalar event fields plus
//! short JSONB arrays for `alternate_names`, `image`, `same_as`,
//! `keywords`, and `in_language`. Repeated relational data lives
//! in child tables: `event_identifiers`, `event_locations`,
//! `event_parties`, `event_offers`, `event_links`, and
//! `event_sub_events`. Organizations and audit log retain their own
//! tables.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// events
// ============================================================================

/// SeaORM entity for the `events` table (the event aggregate root).
pub mod events {
    use super::*;

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
        /// `has_many` event_identifiers.
        #[sea_orm(has_many = "super::event_identifiers::Entity")]
        Identifiers,
        /// `has_many` event_locations.
        #[sea_orm(has_many = "super::event_locations::Entity")]
        Locations,
        /// `has_many` event_parties.
        #[sea_orm(has_many = "super::event_parties::Entity")]
        Parties,
        /// `has_many` event_offers.
        #[sea_orm(has_many = "super::event_offers::Entity")]
        Offers,
        /// `has_many` event_links.
        #[sea_orm(has_many = "super::event_links::Entity")]
        Links,
        /// `has_many` event_sub_events.
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

/// SeaORM entity for the `event_identifiers` child table.
pub mod event_identifiers {
    use super::*;

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

/// SeaORM entity for the `event_locations` child table. The `kind`
/// column discriminates the `Location` variant; only that variant's
/// columns are populated, and `position` preserves ordering.
pub mod event_locations {
    use super::*;

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
        /// "place" | "postal_address" | "virtual" | "text"
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
        /// Latitude (for "place").
        pub latitude: Option<f64>,
        /// Longitude (for "place").
        pub longitude: Option<f64>,
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

/// SeaORM entity for the `event_parties` child table (organizers,
/// performers, attendees, sponsors, funders, contributors).
pub mod event_parties {
    use super::*;

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

/// SeaORM entity for the `event_offers` child table (ticket/pricing tiers).
pub mod event_offers {
    use super::*;

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

/// SeaORM entity for the `event_links` child table (cross-event links).
pub mod event_links {
    use super::*;

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
        /// Serialized `LinkType` (Replaces / ReplacedBy / Refer / Seealso).
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

/// SeaORM entity for the `event_sub_events` child table (schema.org
/// `subEvent` ordering list).
pub mod event_sub_events {
    use super::*;

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
// organizations (and child tables — unchanged from prior shape)
// ============================================================================

/// SeaORM entity for the `organizations` table.
pub mod organizations {
    use super::*;

    /// One organization row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organizations")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Soft-delete / visibility flag.
        pub active: bool,
        /// Organization name.
        pub name: String,
        /// Alternate names / aliases.
        pub alias: Vec<String>,
        /// Organization type labels.
        pub org_type: Vec<String>,
        /// Parent organization id (`partOf`).
        pub part_of: Option<Uuid>,
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

    /// Relations from `organizations` to its child tables.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `has_many` organization_addresses.
        #[sea_orm(has_many = "super::organization_addresses::Entity")]
        Addresses,
        /// `has_many` organization_contacts.
        #[sea_orm(has_many = "super::organization_contacts::Entity")]
        Contacts,
        /// `has_many` organization_identifiers.
        #[sea_orm(has_many = "super::organization_identifiers::Entity")]
        Identifiers,
    }

    impl Related<super::organization_addresses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Addresses.def()
        }
    }
    impl Related<super::organization_contacts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Contacts.def()
        }
    }
    impl Related<super::organization_identifiers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Identifiers.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM entity for the `organization_addresses` child table.
pub mod organization_addresses {
    use super::*;

    /// One postal address belonging to an organization.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_addresses")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization id.
        pub organization_id: Uuid,
        /// Address use/category.
        pub use_type: Option<String>,
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
        /// Whether this is the primary address.
        pub is_primary: bool,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Relation back to the owning organization.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent organization.
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM entity for the `organization_contacts` child table.
pub mod organization_contacts {
    use super::*;

    /// One contact point (phone / email / …) for an organization.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_contacts")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization id.
        pub organization_id: Uuid,
        /// Contact system (phone / email / …).
        pub system: String,
        /// Contact value.
        pub value: String,
        /// Contact use/category.
        pub use_type: Option<String>,
        /// Whether this is the primary contact.
        pub is_primary: bool,
        /// Row creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Row last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Relation back to the owning organization.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent organization.
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// SeaORM entity for the `organization_identifiers` child table.
pub mod organization_identifiers {
    use super::*;

    /// One external identifier belonging to an organization.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_identifiers")]
    pub struct Model {
        /// Primary key (UUID).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization id.
        pub organization_id: Uuid,
        /// Identifier use/category.
        pub use_type: Option<String>,
        /// Serialized identifier type.
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

    /// Relation back to the owning organization.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent organization.
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// audit_log (unchanged)
// ============================================================================

/// SeaORM entity for the `audit_log` table (HIPAA-style audit trail).
pub mod audit_log {
    use super::*;

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
    }

    /// `audit_log` has no relations.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ───────────────────── event_text_values ─────────────────────

/// Tagged table for the Event's parallel string-list properties
/// (alternate_name | image | same_as | keyword | in_language).
pub mod event_text_values {
    use super::*;

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
        #[sea_orm(belongs_to = "super::events::Entity", from = "Column::EventId", to = "super::events::Column::Id")]
        Event,
    }
    impl Related<super::events::Entity> for Entity {
        fn to() -> RelationDef { Relation::Event.def() }
    }
    impl ActiveModelBehavior for ActiveModel {}
}
