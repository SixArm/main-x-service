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

pub mod events {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub active: bool,
        pub name: String,
        pub description: Option<String>,
        pub disambiguating_description: Option<String>,
        pub url: Option<String>,
        #[sea_orm(column_type = "JsonBinary")]
        pub alternate_names: Json,
        #[sea_orm(column_type = "JsonBinary")]
        pub image: Json,
        #[sea_orm(column_type = "JsonBinary")]
        pub same_as: Json,
        #[sea_orm(column_type = "JsonBinary")]
        pub keywords: Json,
        #[sea_orm(column_type = "JsonBinary")]
        pub in_language: Json,
        pub start_date: DateTimeUtc,
        pub end_date: Option<DateTimeUtc>,
        pub door_time: Option<DateTimeUtc>,
        pub duration: Option<String>,
        pub previous_start_date: Option<DateTimeUtc>,
        pub time_zone: Option<String>,
        pub all_day: bool,
        pub event_status: String,
        pub event_attendance_mode: String,
        pub event_type: String,
        pub typical_age_range: Option<String>,
        pub is_accessible_for_free: Option<bool>,
        pub maximum_attendee_capacity: Option<i32>,
        pub maximum_physical_attendee_capacity: Option<i32>,
        pub maximum_virtual_attendee_capacity: Option<i32>,
        pub remaining_attendee_capacity: Option<i32>,
        pub super_event_id: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        pub created_by: Option<String>,
        pub updated_by: Option<String>,
        pub deleted_at: Option<DateTimeUtc>,
        pub deleted_by: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::event_identifiers::Entity")]
        Identifiers,
        #[sea_orm(has_many = "super::event_locations::Entity")]
        Locations,
        #[sea_orm(has_many = "super::event_parties::Entity")]
        Parties,
        #[sea_orm(has_many = "super::event_offers::Entity")]
        Offers,
        #[sea_orm(has_many = "super::event_links::Entity")]
        Links,
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

pub mod event_identifiers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_identifiers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub event_id: Uuid,
        pub use_type: Option<String>,
        pub identifier_type: String,
        pub system: String,
        pub value: String,
        pub assigner: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod event_locations {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_locations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub event_id: Uuid,
        pub position: i32,
        /// "place" | "postal_address" | "virtual" | "text"
        pub kind: String,
        /// For "place": optional external place-service id.
        pub place_id: Option<Uuid>,
        /// For "place": display name. For "virtual": optional name.
        /// For "text": the text value.
        pub name: Option<String>,
        pub line1: Option<String>,
        pub line2: Option<String>,
        pub city: Option<String>,
        pub state: Option<String>,
        pub postal_code: Option<String>,
        pub country: Option<String>,
        pub latitude: Option<f64>,
        pub longitude: Option<f64>,
        pub url: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod event_parties {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_parties")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub event_id: Uuid,
        pub position: i32,
        /// "organizer" | "performer" | "attendee" | "sponsor" |
        /// "funder" | "contributor"
        pub role: String,
        /// "person" | "organization"
        pub party_kind: String,
        /// Optional external person/org-service id.
        pub party_id: Option<Uuid>,
        pub name: String,
        pub email: Option<String>,
        pub url: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod event_offers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_offers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub event_id: Uuid,
        pub position: i32,
        pub name: Option<String>,
        #[sea_orm(column_type = "Decimal(Some((12, 4)))")]
        pub price: Option<bigdecimal::BigDecimal>,
        pub price_currency: Option<String>,
        pub url: Option<String>,
        pub availability: Option<String>,
        pub valid_from: Option<DateTimeUtc>,
        pub valid_through: Option<DateTimeUtc>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod event_links {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_links")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub event_id: Uuid,
        pub other_event_id: Uuid,
        pub link_type: String,
        pub created_at: DateTimeUtc,
        pub created_by: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod event_sub_events {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_sub_events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub event_id: Uuid,
        pub sub_event_id: Uuid,
        pub position: i32,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod organizations {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organizations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub active: bool,
        pub name: String,
        pub alias: Vec<String>,
        pub org_type: Vec<String>,
        pub part_of: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        pub created_by: Option<String>,
        pub updated_by: Option<String>,
        pub deleted_at: Option<DateTimeUtc>,
        pub deleted_by: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::organization_addresses::Entity")]
        Addresses,
        #[sea_orm(has_many = "super::organization_contacts::Entity")]
        Contacts,
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

pub mod organization_addresses {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_addresses")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub use_type: Option<String>,
        pub line1: Option<String>,
        pub line2: Option<String>,
        pub city: Option<String>,
        pub state: Option<String>,
        pub postal_code: Option<String>,
        pub country: Option<String>,
        pub is_primary: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod organization_contacts {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_contacts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub system: String,
        pub value: String,
        pub use_type: Option<String>,
        pub is_primary: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod organization_identifiers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_identifiers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub use_type: Option<String>,
        pub identifier_type: String,
        pub system: String,
        pub value: String,
        pub assigner: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
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

pub mod audit_log {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub timestamp: DateTimeUtc,
        pub user_id: Option<String>,
        pub action: String,
        pub entity_type: String,
        pub entity_id: Uuid,
        pub old_values: Option<Json>,
        pub new_values: Option<Json>,
        pub ip_address: Option<String>,
        pub user_agent: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
