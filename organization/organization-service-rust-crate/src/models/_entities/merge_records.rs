//! `SeaORM` Entity — `merge_records`. One row per record-merge: which
//! duplicate folded into which survivor, with a snapshot of the
//! transferred payload.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `merge_records` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One merge-history row (`SeaORM`-generated). Mirrors the
/// `merge_records` table from `m20220101_000003_merge_records`; one row
/// is written per successful merge for auditability.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "merge_records")]
pub struct Model {
    /// Row insert time (UTC) — when the merge happened.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-modification time (UTC); merge rows are append-only.
    pub updated_at: DateTimeWithTimeZone,
    /// Auto-increment surrogate primary key; also the recency order key.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// The surviving (main) organization's `pid` — the record kept.
    pub main_pid: Uuid,
    /// The merged-away duplicate's `pid` — now soft-deleted.
    pub duplicate_pid: Uuid,
    /// Optional operator-supplied free-text reason for the merge.
    pub reason: Option<String>,
    /// The actor that performed the merge (bearer `sub`), else `None`.
    pub actor: Option<String>,
    /// JSON snapshot of the duplicate's payload at merge time, so the
    /// merged-away data is recoverable from the history.
    pub transferred: Option<Json>,
}

/// `SeaORM` relation enum for `merge_records`. Rows reference
/// organizations only by `pid` value (no FK), so no relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
