//! `SeaORM` Entity — `bulk_jobs`. Durable state for one asynchronous bulk
//! operation (`agents/share/bulk-import-export.md` §3).

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `bulk_jobs` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One bulk job: its kind, status, counters, and artifact references.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bulk_jobs")]
pub struct Model {
    /// Job id — also the `$export-status` path segment.
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    /// `import` | `export`.
    pub kind: String,
    /// The entity the job operates on.
    pub entity: String,
    /// Output/input format (`ndjson` for FHIR Bulk Data).
    pub format: String,
    /// `queued` | `running` | `completed` | `failed` | `cancelled`.
    pub status: String,
    /// Request parameters, so a job stays interpretable after the fact.
    pub params: Json,
    pub rows_total: Option<i64>,
    pub rows_processed: i64,
    pub rows_created: i64,
    pub rows_upserted: i64,
    pub rows_to_review: i64,
    pub rows_errored: i64,
    /// The submitting caller's `sub`, when a verified token was presented.
    pub actor: Option<String>,
    /// Client-supplied key making a retried submit return the same job.
    pub idempotency_key: Option<String>,
    pub input_url: Option<String>,
    /// Reference to the output artifact, set when the job completes.
    pub result_url: Option<String>,
    pub error_report_url: Option<String>,
    /// Failure reason when `status = failed`.
    pub error: Option<String>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    /// When the row and its artifacts may be swept.
    pub expires_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
