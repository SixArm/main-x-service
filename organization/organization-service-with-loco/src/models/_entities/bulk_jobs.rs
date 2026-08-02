//! `SeaORM` Entity — `bulk_jobs`. Durable state for asynchronous bulk
//! import/export jobs (BLK-5; `agents/share/bulk-import-export.md` §3).

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `bulk_jobs` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One persisted bulk-job row: kind + format + status, per-row counts,
/// and the artifact references (input, result, error report). Mirrors
/// the `bulk_jobs` table from `m20260803_000002_bulk_jobs`.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bulk_jobs")]
pub struct Model {
    /// Application-assigned primary key (the job id).
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// `import` or `export`.
    pub kind: String,
    /// The entity name (`organization`).
    pub entity: String,
    /// File format token (`jsonl` | `csv` — BLK-5 scope; no Parquet).
    pub format: String,
    /// `queued` | `running` | `completed` | `completed_with_errors` | `failed`.
    pub status: String,
    /// Free-form job parameters (dry-run flag, export filter, masking
    /// profile, …).
    pub params: Json,
    /// Total record rows seen, once known.
    pub rows_total: Option<i64>,
    /// Rows processed so far.
    pub rows_processed: i64,
    /// Rows inserted as new records.
    pub rows_created: i64,
    /// Rows upserted onto an existing record (idempotent re-import).
    pub rows_upserted: i64,
    /// Rows routed to the duplicate review queue (keyless-row detection).
    pub rows_to_review: i64,
    /// Rows that failed validation/parse/persist.
    pub rows_errored: i64,
    /// Acting user pid (bearer `sub`), if any.
    pub actor: Option<String>,
    /// Client-supplied idempotency key, if any (SEC-B9).
    pub idempotency_key: Option<String>,
    /// Artifact reference for the uploaded source file.
    pub input_url: Option<String>,
    /// Artifact reference for the export output.
    pub result_url: Option<String>,
    /// Artifact reference for the downloadable per-row error report.
    pub error_report_url: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTimeWithTimeZone,
    /// Last-update timestamp.
    pub updated_at: DateTimeWithTimeZone,
    /// When the job row and its artifacts may be swept (SEC-B4).
    pub expires_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relation enum for `bulk_jobs`. Standalone (referenced only by
/// application-level `entity`/`actor` strings, no FK), so no relations.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
