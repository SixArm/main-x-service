//! SeaORM entity definitions for the Thing Service.
//!
//! Fully normalized: the Thing aggregate's repeating collections
//! (alternate_names, identifiers, images, same_as) live in dedicated child
//! tables joined by `thing_id`, rather than JSONB columns. JSONB is used
//! only for opaque snapshots (audit old/new values, merge transferred_data).
//! Timestamps use the `time` crate at the SeaORM boundary; the repository
//! converts to/from `jiff::Timestamp`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// The `things` table: the core thing record (scalar fields only).
pub mod things {
    use super::*;

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
        /// same_as URLs.
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
    use super::*;

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

/// The `thing_identifiers` table (schema.org PropertyValue shape).
pub mod thing_identifiers {
    use super::*;

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
    use super::*;

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
    use super::*;

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

/// The `thing_merge_records` table (transferred_data is an opaque snapshot).
pub mod thing_merge_records {
    use super::*;

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

/// The `audit_log` table (old/new values are opaque snapshots).
pub mod audit_log {
    use super::*;

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
    }

    /// No outbound relations.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
