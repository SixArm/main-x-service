//! `SeaORM` entity definitions for the Place Service.
//!
//! Fully normalized: the 0..1 nested objects (`PostalAddress`,
//! `GeoCoordinates`) are flattened onto the `places` row; the repeating
//! collections (keywords, identifiers, `amenity_features`, `opening_hours`)
//! live in dedicated child tables joined by `place_id`. JSONB is used only
//! for opaque snapshots (audit old/new values, merge `transferred_data`).

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The `places` table: core place record + flattened address/geo.
pub mod places {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

    /// A row in `places`. Soft-delete is via `is_deleted`/`deleted_at`.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "places")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
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
        /// `GeoCoordinates.latitude` (flattened).
        pub geo_latitude: Option<bigdecimal::BigDecimal>,
        /// `GeoCoordinates.longitude` (flattened).
        pub geo_longitude: Option<bigdecimal::BigDecimal>,
        /// `GeoCoordinates.elevation` (flattened).
        pub geo_elevation: Option<bigdecimal::BigDecimal>,
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
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// `has_many` child collections.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// keywords.
        #[sea_orm(has_many = "super::place_keywords::Entity")]
        Keywords,
        /// identifiers.
        #[sea_orm(has_many = "super::place_identifiers::Entity")]
        Identifiers,
        /// amenity features.
        #[sea_orm(has_many = "super::place_amenity_features::Entity")]
        AmenityFeatures,
        /// opening hours.
        #[sea_orm(has_many = "super::place_opening_hours::Entity")]
        OpeningHours,
    }

    impl Related<super::place_keywords::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Keywords.def()
        }
    }
    impl Related<super::place_identifiers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Identifiers.def()
        }
    }
    impl Related<super::place_amenity_features::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::AmenityFeatures.def()
        }
    }
    impl Related<super::place_opening_hours::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::OpeningHours.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// The `place_keywords` table.
pub mod place_keywords {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        Uuid,
    };

    /// One keyword for a place.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "place_keywords")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning place id (FK).
        pub place_id: Uuid,
        /// The keyword.
        pub keyword: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent place.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `places`.
        #[sea_orm(
            belongs_to = "super::places::Entity",
            from = "Column::PlaceId",
            to = "super::places::Column::Id"
        )]
        Place,
    }
    impl Related<super::places::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Place.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `place_identifiers` table.
pub mod place_identifiers {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        Uuid,
    };

    /// One external identifier for a place.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "place_identifiers")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning place id (FK).
        pub place_id: Uuid,
        /// Identifier scheme tag (or `custom`).
        pub identifier_type: String,
        /// Free-text label for the `custom` scheme.
        pub custom_label: Option<String>,
        /// The identifier value.
        pub value: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent place.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `places`.
        #[sea_orm(
            belongs_to = "super::places::Entity",
            from = "Column::PlaceId",
            to = "super::places::Column::Id"
        )]
        Place,
    }
    impl Related<super::places::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Place.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `place_amenity_features` table.
pub mod place_amenity_features {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        Uuid,
    };

    /// One amenity feature (key/value) for a place.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "place_amenity_features")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning place id (FK).
        pub place_id: Uuid,
        /// Feature name.
        pub name: String,
        /// Feature value, if any.
        pub value: Option<String>,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent place.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `places`.
        #[sea_orm(
            belongs_to = "super::places::Entity",
            from = "Column::PlaceId",
            to = "super::places::Column::Id"
        )]
        Place,
    }
    impl Related<super::places::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Place.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `place_opening_hours` table.
pub mod place_opening_hours {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        Uuid,
    };

    /// One opening-hours specification for a place.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "place_opening_hours")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning place id (FK).
        pub place_id: Uuid,
        /// Day of week tag (monday..sunday).
        pub day_of_week: String,
        /// Opening time "HH:MM".
        pub opens: String,
        /// Closing time "HH:MM".
        pub closes: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent place.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `places`.
        #[sea_orm(
            belongs_to = "super::places::Entity",
            from = "Column::PlaceId",
            to = "super::places::Column::Id"
        )]
        Place,
    }
    impl Related<super::places::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Place.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `place_merge_records` table (`transferred_data` is an opaque snapshot).
pub mod place_merge_records {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, Json, PrimaryKeyTrait, Serialize, TimeDateTimeWithTimeZone, Uuid,
    };

    /// A row recording the fold of a duplicate place into a main place.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "place_merge_records")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Surviving place id.
        pub main_place_id: Uuid,
        /// Absorbed (soft-deleted) place id.
        pub duplicate_place_id: Uuid,
        /// Free-text reason for the merge.
        pub merge_reason: Option<String>,
        /// Snapshot of data transferred from duplicate to main (opaque).
        pub transferred_data: Option<Json>,
        /// When the merge happened.
        pub merged_at: TimeDateTimeWithTimeZone,
    }

    /// No outbound relations.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// The `audit_log` table (old/new values are opaque snapshots).
pub mod audit_log {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, Json, PrimaryKeyTrait, Serialize, TimeDateTimeWithTimeZone, Uuid,
    };

    /// One audit row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
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
        pub old_values: Option<Json>,
        /// Post-change snapshot (opaque).
        pub new_values: Option<Json>,
        /// When the action occurred.
        pub created_at: TimeDateTimeWithTimeZone,
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

    /// No outbound relations.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// `SeaORM` entity for the `event_outbox` table — the transactional-outbox
/// hand-off buffer for the durable event bus (Phase 2;
/// `agents/share/event-bus.md` §3). One row is written **in the same
/// transaction** as the entity mutation, so a committed change always has
/// its event and vice versa; a relay worker (Phase 3, roadmap) drains
/// unpublished rows to Fluvio and stamps `published_at`.
pub mod event_outbox {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, Json, PrimaryKeyTrait, Serialize, TimeDateTimeWithTimeZone, Uuid,
    };

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
        /// The entity name (`place`).
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
