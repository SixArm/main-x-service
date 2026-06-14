//! `SeaORM` Entity — `organizations`. The full
//! `organization_matcher::Organization` payload is stored in `data`
//! (JSONB); `pid` and `name` are denormalised for lookup and listing.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `organizations` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted organization row (`SeaORM`-generated `DeriveEntityModel`).
/// Mirrors the `organizations` table from the
/// `m20220101_000001_organizations` migration; the canonical payload
/// lives in `data` while `pid`/`name` are denormalised for lookup.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "organizations")]
pub struct Model {
    /// Row insert time (UTC), set by `SeaORM` on create.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-modification time (UTC), bumped by `SeaORM` on update.
    pub updated_at: DateTimeWithTimeZone,
    /// Auto-increment surrogate primary key (internal; never exposed on
    /// the wire — used for stable `ORDER BY id` listing).
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Public, externally-stable id (UUID v4). The only identifier the
    /// REST API references; unique per the migration.
    #[sea_orm(unique)]
    pub pid: Uuid,
    /// Denormalised `Organization.name`, kept in its own column so list
    /// and `ILIKE` name search avoid touching the JSON payload.
    pub name: String,
    /// The full `organization_matcher::Organization` payload, stored
    /// verbatim as JSONB; deserialized back via `Model::to_org`.
    pub data: Json,
    /// Soft-delete liveness flag: `true` while active, `false` once
    /// soft-deleted (paired with `deleted_at`).
    pub active: bool,
    /// Soft-delete timestamp: `None` while active, set when soft-deleted.
    /// Active-row queries filter on `deleted_at IS NULL`.
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relation enum for `organizations`. The table is standalone
/// (the JSON payload holds all associations), so there are no relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
