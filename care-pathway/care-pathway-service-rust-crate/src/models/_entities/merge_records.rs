//! `SeaORM` Entity — `merge_records`. One row per record-merge: which
//! duplicate folded into which survivor, by whom, with a snapshot of the
//! transferred payload.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `merge_records` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted merge-history row: one duplicate folded into one survivor.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "merge_records")]
pub struct Model {
    /// Row creation timestamp (when the merge happened).
    pub created_at: DateTimeWithTimeZone,
    /// Row last-update timestamp.
    pub updated_at: DateTimeWithTimeZone,
    /// Internal auto-increment primary key.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The surviving (main) pathway's `pid`.
    pub main_pid: Uuid,
    /// The merged-away (duplicate) pathway's `pid`, now soft-deleted.
    pub duplicate_pid: Uuid,
    /// Optional operator-supplied reason for the merge.
    pub reason: Option<String>,
    /// The acting user's `sub` (token `pid`), or `None` when unauthenticated.
    pub actor: Option<String>,
    /// Snapshot of the duplicate's payload at merge time (JSON).
    pub transferred: Option<Json>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
