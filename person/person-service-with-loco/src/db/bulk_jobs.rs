//! `bulk_jobs` persistence helpers (`agents/share/bulk-import-export.md`
//! §3).
//!
//! Thin data-access helpers over the [`bulk_jobs`](crate::db::models::bulk_jobs)
//! SeaORM entity: create/enqueue a job, load one by id, and update its
//! status + per-row counts + artifact references as the background worker
//! progresses. The bulk **logic** lives in [`crate::bulk`]; this module
//! only reads and writes the row.

use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::bulk::pipeline::ImportOutcome;
use crate::bulk::{BulkFormat, BulkKind, JobStatus};
use crate::db::models::bulk_jobs;

pub use bulk_jobs::Model;

/// The fields needed to enqueue a new bulk job.
#[derive(Debug, Clone)]
pub struct NewBulkJob {
    /// Import or export.
    pub kind: BulkKind,
    /// File format (`jsonl` in step 1).
    pub format: BulkFormat,
    /// Free-form parameters (dry-run flag, export filter, …).
    pub params: serde_json::Value,
    /// Acting user pid (bearer `sub`), if any.
    pub actor: Option<String>,
    /// Client-supplied idempotency key, if any.
    pub idempotency_key: Option<String>,
    /// Reference to the uploaded input artifact (import only).
    pub input_url: Option<String>,
}

impl NewBulkJob {
    /// A JSONL **import** job with the given params and actor. The input
    /// artifact is attached afterwards via [`set_input_url`].
    #[must_use]
    pub fn import(params: serde_json::Value, actor: Option<String>) -> Self {
        Self {
            kind: BulkKind::Import,
            format: BulkFormat::Jsonl,
            params,
            actor,
            idempotency_key: None,
            input_url: None,
        }
    }

    /// A JSONL **export** job with the given filter params and actor.
    #[must_use]
    pub fn export(params: serde_json::Value, actor: Option<String>) -> Self {
        Self {
            kind: BulkKind::Export,
            format: BulkFormat::Jsonl,
            params,
            actor,
            idempotency_key: None,
            input_url: None,
        }
    }
}

/// Insert a new `queued` bulk job and return its persisted row.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the insert fails.
pub async fn create(db: &DatabaseConnection, job: NewBulkJob) -> Result<Model> {
    let now = OffsetDateTime::now_utc();
    let model = bulk_jobs::ActiveModel {
        id: Set(Uuid::new_v4()),
        kind: Set(job.kind.as_str().to_string()),
        entity: Set(crate::streaming::ENTITY.to_string()),
        format: Set(job.format.as_str().to_string()),
        status: Set(JobStatus::Queued.as_str().to_string()),
        params: Set(job.params),
        rows_total: Set(None),
        rows_processed: Set(0),
        rows_created: Set(0),
        rows_upserted: Set(0),
        rows_to_review: Set(0),
        rows_errored: Set(0),
        actor: Set(job.actor),
        idempotency_key: Set(job.idempotency_key),
        input_url: Set(job.input_url),
        result_url: Set(None),
        error_report_url: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        expires_at: Set(None),
    };
    Ok(model.insert(db).await?)
}

/// Load one bulk job by id, or `None` if absent.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>> {
    Ok(bulk_jobs::Entity::find_by_id(id).one(db).await?)
}

/// List the most recent bulk jobs (newest first), capped at `limit`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn list_recent(db: &DatabaseConnection, limit: u64) -> Result<Vec<Model>> {
    use sea_orm::{QueryOrder, QuerySelect};
    Ok(bulk_jobs::Entity::find()
        .order_by_desc(bulk_jobs::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?)
}

/// Attach the uploaded input artifact reference to an import job.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn set_input_url(db: &DatabaseConnection, id: Uuid, input_url: String) -> Result<()> {
    let mut active: bulk_jobs::ActiveModel = load_active(db, id).await?;
    active.input_url = Set(Some(input_url));
    active.updated_at = Set(OffsetDateTime::now_utc());
    active.update(db).await?;
    Ok(())
}

/// Transition a job to a new status, stamping `updated_at`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn set_status(db: &DatabaseConnection, id: Uuid, status: JobStatus) -> Result<()> {
    let mut active: bulk_jobs::ActiveModel = load_active(db, id).await?;
    active.status = Set(status.as_str().to_string());
    active.updated_at = Set(OffsetDateTime::now_utc());
    active.update(db).await?;
    Ok(())
}

/// Record the outcome of an import run: final status, per-row counts, and
/// the error-report reference.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn finish_import(
    db: &DatabaseConnection,
    id: Uuid,
    outcome: &ImportOutcome,
    error_report_url: Option<String>,
) -> Result<()> {
    let status = if outcome.rows_errored > 0 {
        JobStatus::CompletedWithErrors
    } else {
        JobStatus::Completed
    };
    let mut active: bulk_jobs::ActiveModel = load_active(db, id).await?;
    active.status = Set(status.as_str().to_string());
    active.rows_total = Set(Some(i64_of(outcome.rows_total)));
    active.rows_processed = Set(i64_of(outcome.rows_total));
    active.rows_created = Set(i64_of(outcome.rows_created));
    active.rows_upserted = Set(i64_of(outcome.rows_upserted));
    active.rows_to_review = Set(i64_of(outcome.rows_to_review));
    active.rows_errored = Set(i64_of(outcome.rows_errored));
    active.error_report_url = Set(error_report_url);
    active.updated_at = Set(OffsetDateTime::now_utc());
    active.update(db).await?;
    Ok(())
}

/// Record the outcome of an export run: `completed` status, row count,
/// and the output reference.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn finish_export(
    db: &DatabaseConnection,
    id: Uuid,
    rows_total: u64,
    result_url: String,
) -> Result<()> {
    let mut active: bulk_jobs::ActiveModel = load_active(db, id).await?;
    active.status = Set(JobStatus::Completed.as_str().to_string());
    active.rows_total = Set(Some(i64_of(rows_total)));
    active.rows_processed = Set(i64_of(rows_total));
    active.result_url = Set(Some(result_url));
    active.updated_at = Set(OffsetDateTime::now_utc());
    active.update(db).await?;
    Ok(())
}

/// Load a job as an `ActiveModel` for update, erroring if it is gone.
async fn load_active(db: &DatabaseConnection, id: Uuid) -> Result<bulk_jobs::ActiveModel> {
    let model = find_by_id(db, id)
        .await?
        .ok_or_else(|| crate::Error::Internal(format!("bulk job {id} not found")))?;
    Ok(model.into())
}

/// Saturating `u64` → `i64` for count columns.
fn i64_of(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}
