//! `SeaORM` entity definitions for the Thing Service.
//!
//! Fully normalized: the Thing aggregate's repeating collections
//! (`alternate_names`, identifiers, images, `same_as`) live in dedicated child
//! tables joined by `thing_id`, rather than JSONB columns. JSONB is used
//! only for opaque snapshots (audit old/new values, merge `transferred_data`).
//! Timestamps use the `time` crate at the `SeaORM` boundary; the repository
//! converts to/from `chrono::DateTime<Utc>`.

use serde::{Deserialize, Serialize};

/// The `things` table: the core thing record (scalar fields only).
pub mod things {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in `things`. Soft-delete is via `is_deleted`/`deleted_at`.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "things")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Primary name.
        pub name: String,
        /// Free-text description.
        pub description: Option<String>,
        /// Short distinguishing description.
        pub disambiguating_description: Option<String>,
        /// schema.org `additionalType` URL.
        pub additional_type: Option<String>,
        /// Canonical URL.
        pub url: Option<String>,
        /// `mainEntityOfPage` URL.
        pub main_entity_of_page: Option<String>,
        /// Owner reference.
        pub owner: Option<String>,
        /// `subjectOf` reference.
        pub subject_of: Option<String>,
        /// `potentialAction` reference.
        pub potential_action: Option<String>,
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
        /// alternate names.
        #[sea_orm(has_many = "super::thing_alternate_names::Entity")]
        AlternateNames,
        /// identifiers.
        #[sea_orm(has_many = "super::thing_identifiers::Entity")]
        Identifiers,
        /// images.
        #[sea_orm(has_many = "super::thing_images::Entity")]
        Images,
        /// `same_as` URLs.
        #[sea_orm(has_many = "super::thing_same_as::Entity")]
        SameAs,
    }

    impl Related<super::thing_alternate_names::Entity> for Entity {
        /// Relation from `things` to its `thing_alternate_names` children.
        fn to() -> RelationDef {
            Relation::AlternateNames.def()
        }
    }
    impl Related<super::thing_identifiers::Entity> for Entity {
        /// Relation from `things` to its `thing_identifiers` children.
        fn to() -> RelationDef {
            Relation::Identifiers.def()
        }
    }
    impl Related<super::thing_images::Entity> for Entity {
        /// Relation from `things` to its `thing_images` children.
        fn to() -> RelationDef {
            Relation::Images.def()
        }
    }
    impl Related<super::thing_same_as::Entity> for Entity {
        /// Relation from `things` to its `thing_same_as` children.
        fn to() -> RelationDef {
            Relation::SameAs.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

/// The `thing_alternate_names` table.
pub mod thing_alternate_names {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One alternate name for a thing.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "thing_alternate_names")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning thing id (FK).
        pub thing_id: Uuid,
        /// The alternate name.
        pub name: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent thing.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `things`.
        #[sea_orm(
            belongs_to = "super::things::Entity",
            from = "Column::ThingId",
            to = "super::things::Column::Id"
        )]
        Thing,
    }
    impl Related<super::things::Entity> for Entity {
        /// Inverse relation: this child belongs to its parent `things` row.
        fn to() -> RelationDef {
            Relation::Thing.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `thing_identifiers` table (schema.org `PropertyValue` shape).
pub mod thing_identifiers {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One typed identifier for a thing.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "thing_identifiers")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning thing id (FK).
        pub thing_id: Uuid,
        /// Identifier scheme (doi|isbn|…|custom).
        pub property_id: String,
        /// Free-text label for the `custom` scheme.
        pub custom_label: Option<String>,
        /// The identifier value.
        pub value: String,
        /// Optional display name.
        pub name: Option<String>,
        /// Optional resolving URL.
        pub url: Option<String>,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent thing.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `things`.
        #[sea_orm(
            belongs_to = "super::things::Entity",
            from = "Column::ThingId",
            to = "super::things::Column::Id"
        )]
        Thing,
    }
    impl Related<super::things::Entity> for Entity {
        /// Inverse relation: this child belongs to its parent `things` row.
        fn to() -> RelationDef {
            Relation::Thing.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `thing_images` table.
pub mod thing_images {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One image URL for a thing.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "thing_images")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning thing id (FK).
        pub thing_id: Uuid,
        /// The image URL.
        pub url: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent thing.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `things`.
        #[sea_orm(
            belongs_to = "super::things::Entity",
            from = "Column::ThingId",
            to = "super::things::Column::Id"
        )]
        Thing,
    }
    impl Related<super::things::Entity> for Entity {
        /// Inverse relation: this child belongs to its parent `things` row.
        fn to() -> RelationDef {
            Relation::Thing.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `thing_same_as` table.
pub mod thing_same_as {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One `sameAs` authoritative URL for a thing.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "thing_same_as")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning thing id (FK).
        pub thing_id: Uuid,
        /// The URL.
        pub url: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent thing.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `things`.
        #[sea_orm(
            belongs_to = "super::things::Entity",
            from = "Column::ThingId",
            to = "super::things::Column::Id"
        )]
        Thing,
    }
    impl Related<super::things::Entity> for Entity {
        /// Inverse relation: this child belongs to its parent `things` row.
        fn to() -> RelationDef {
            Relation::Thing.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `thing_merge_records` table (`transferred_data` is an opaque snapshot).
pub mod thing_merge_records {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row recording the fold of a duplicate thing into a main thing.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "thing_merge_records")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Surviving thing id.
        pub main_thing_id: Uuid,
        /// Absorbed (soft-deleted) thing id.
        pub duplicate_thing_id: Uuid,
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
        /// The entity name (`thing`).
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

/// The `audit_log` table (old/new values are opaque snapshots).
pub mod audit_log {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

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
