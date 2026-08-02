//! `bulk_jobs` model — CRUD/status helpers over the async bulk
//! import/export job table (BLK-5; `agents/share/bulk-import-export.md`
//! §3).
//!
//! Thin data-access helpers: create/enqueue a job, load one by id or by
//! idempotency key, and update its status + per-row counts + artifact
//! references as the background worker ([`crate::bulk::worker`])
//! progresses. The bulk **logic** lives in [`crate::bulk::pipeline`];
//! this module only reads and writes the row.

use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::bulk_jobs::{self, ActiveModel, Entity, Model};
use crate::bulk::pipeline::ImportOutcome;
use crate::bulk::{BulkFormat, BulkKind, JobStatus};

/// Default active-model lifecycle hooks (no custom create/update logic).
impl ActiveModelBehavior for super::_entities::bulk_jobs::ActiveModel {}

/// SEC-B4 — the job/artifact expiry stamp: `created_at` plus
/// [`crate::bulk::BULK_ARTIFACT_TTL_SECS`].
fn expires_at_of(
    created_at: chrono::DateTime<chrono::FixedOffset>,
) -> chrono::DateTime<chrono::FixedOffset> {
    created_at + chrono::Duration::seconds(crate::bulk::BULK_ARTIFACT_TTL_SECS)
}

/// Saturating `u64` → `i64` for count columns.
fn i64_of(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// The fields needed to enqueue a new bulk job.
#[derive(Debug, Clone)]
pub struct NewBulkJob {
    /// Import or export.
    pub kind: BulkKind,
    /// File format (`jsonl` or `csv`).
    pub format: BulkFormat,
    /// Free-form parameters (dry-run flag, export filter, masking
    /// profile, …).
    pub params: serde_json::Value,
    /// Acting user pid (bearer `sub`), if any.
    pub actor: Option<String>,
    /// Client-supplied idempotency key, if any.
    pub idempotency_key: Option<String>,
}

impl NewBulkJob {
    /// An **import** job in `format` with the given params and actor. The
    /// input artifact is attached afterwards via
    /// [`Model::set_input_url`].
    #[must_use]
    pub fn import(format: BulkFormat, params: serde_json::Value, actor: Option<String>) -> Self {
        Self {
            kind: BulkKind::Import,
            format,
            params,
            actor,
            idempotency_key: None,
        }
    }

    /// An **export** job in `format` with the given filter params and actor.
    #[must_use]
    pub fn export(format: BulkFormat, params: serde_json::Value, actor: Option<String>) -> Self {
        Self {
            kind: BulkKind::Export,
            format,
            params,
            actor,
            idempotency_key: None,
        }
    }

    /// Attach a client-supplied idempotency key (SEC-B9), trimmed; a blank
    /// key is treated as absent. A retried submit carrying the same key
    /// dedupes to the same job ([`Model::create_or_get_idempotent`]).
    #[must_use]
    pub fn with_idempotency_key(mut self, key: Option<String>) -> Self {
        self.idempotency_key = key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty());
        self
    }
}

impl Model {
    /// Insert a new `queued` bulk job and return its persisted row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn create(db: &DatabaseConnection, job: NewBulkJob) -> ModelResult<Self> {
        let now: chrono::DateTime<chrono::FixedOffset> = Utc::now().into();
        let model = bulk_jobs::ActiveModel {
            id: ActiveValue::set(Uuid::new_v4()),
            kind: ActiveValue::set(job.kind.as_str().to_string()),
            entity: ActiveValue::set(crate::streaming::ENTITY.to_string()),
            format: ActiveValue::set(job.format.as_str().to_string()),
            status: ActiveValue::set(JobStatus::Queued.as_str().to_string()),
            params: ActiveValue::set(job.params),
            rows_total: ActiveValue::set(None),
            rows_processed: ActiveValue::set(0),
            rows_created: ActiveValue::set(0),
            rows_upserted: ActiveValue::set(0),
            rows_to_review: ActiveValue::set(0),
            rows_errored: ActiveValue::set(0),
            actor: ActiveValue::set(job.actor),
            idempotency_key: ActiveValue::set(job.idempotency_key),
            input_url: ActiveValue::set(None),
            result_url: ActiveValue::set(None),
            error_report_url: ActiveValue::set(None),
            created_at: ActiveValue::set(now),
            updated_at: ActiveValue::set(now),
            expires_at: ActiveValue::set(Some(expires_at_of(now))),
        };
        Ok(model.insert(db).await?)
    }

    /// Load one bulk job by id, or `None` if absent.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> ModelResult<Option<Self>> {
        Ok(bulk_jobs::Entity::find_by_id(id).one(db).await?)
    }

    /// Find an existing job for this entity + `kind` bearing
    /// `idempotency_key` (SEC-B9), or `None`. Matches the `UNIQUE (entity,
    /// kind, idempotency_key)` constraint so a retried submit resolves to
    /// the original job.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_idempotency_key(
        db: &DatabaseConnection,
        kind: BulkKind,
        key: &str,
    ) -> ModelResult<Option<Self>> {
        Ok(bulk_jobs::Entity::find()
            .filter(bulk_jobs::Column::Entity.eq(crate::streaming::ENTITY))
            .filter(bulk_jobs::Column::Kind.eq(kind.as_str()))
            .filter(bulk_jobs::Column::IdempotencyKey.eq(key))
            .one(db)
            .await?)
    }

    /// Create a job **idempotently** (SEC-B9): if it carries an
    /// idempotency key that already names a job (this entity + kind),
    /// return that existing job (`reused = true`) instead of inserting a
    /// duplicate — so a retried submit neither re-runs the work nor
    /// creates a second row. A key-less job always inserts. The `UNIQUE
    /// (entity, kind, idempotency_key)` constraint backstops the
    /// check-then-insert race: on a unique violation the existing row is
    /// re-fetched and returned.
    ///
    /// # Errors
    ///
    /// When the query/insert fails for a reason other than a losing
    /// idempotency race.
    pub async fn create_or_get_idempotent(
        db: &DatabaseConnection,
        job: NewBulkJob,
    ) -> ModelResult<(Self, bool)> {
        if let Some(key) = job.idempotency_key.clone() {
            if let Some(existing) = Self::find_by_idempotency_key(db, job.kind, &key).await? {
                return Ok((existing, true));
            }
            let kind = job.kind;
            return match Self::create(db, job).await {
                Ok(model) => Ok((model, false)),
                Err(insert_err) => {
                    // Possible unique-violation race: a concurrent submit
                    // with the same key won. Re-check and return the winner.
                    match Self::find_by_idempotency_key(db, kind, &key).await? {
                        Some(existing) => Ok((existing, true)),
                        None => Err(insert_err),
                    }
                }
            };
        }
        Ok((Self::create(db, job).await?, false))
    }

    /// List the most recent bulk jobs (newest first), capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
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
    /// When the update fails (including an unknown `id`).
    pub async fn set_input_url(
        db: &DatabaseConnection,
        id: Uuid,
        input_url: String,
    ) -> ModelResult<()> {
        let mut active = Self::load_active(db, id).await?;
        active.input_url = ActiveValue::set(Some(input_url));
        active.updated_at = ActiveValue::set(Utc::now().into());
        active.update(db).await?;
        Ok(())
    }

    /// Transition a job to a new status, stamping `updated_at`.
    ///
    /// # Errors
    ///
    /// When the update fails (including an unknown `id`).
    pub async fn set_status(
        db: &DatabaseConnection,
        id: Uuid,
        status: JobStatus,
    ) -> ModelResult<()> {
        let mut active = Self::load_active(db, id).await?;
        active.status = ActiveValue::set(status.as_str().to_string());
        active.updated_at = ActiveValue::set(Utc::now().into());
        active.update(db).await?;
        Ok(())
    }

    /// Record the outcome of an import run: final status, per-row counts,
    /// and the error-report reference.
    ///
    /// # Errors
    ///
    /// When the update fails (including an unknown `id`).
    pub async fn finish_import(
        db: &DatabaseConnection,
        id: Uuid,
        outcome: &ImportOutcome,
        error_report_url: Option<String>,
    ) -> ModelResult<()> {
        let status = if outcome.rows_errored > 0 {
            JobStatus::CompletedWithErrors
        } else {
            JobStatus::Completed
        };
        let mut active = Self::load_active(db, id).await?;
        active.status = ActiveValue::set(status.as_str().to_string());
        active.rows_total = ActiveValue::set(Some(i64_of(outcome.rows_total)));
        active.rows_processed = ActiveValue::set(i64_of(outcome.rows_total));
        active.rows_created = ActiveValue::set(i64_of(outcome.rows_created));
        active.rows_upserted = ActiveValue::set(i64_of(outcome.rows_upserted));
        active.rows_to_review = ActiveValue::set(i64_of(outcome.rows_to_review));
        active.rows_errored = ActiveValue::set(i64_of(outcome.rows_errored));
        active.error_report_url = ActiveValue::set(error_report_url);
        active.updated_at = ActiveValue::set(Utc::now().into());
        active.update(db).await?;
        Ok(())
    }

    /// Record the outcome of an export run: `completed` status, row
    /// count, and the output reference.
    ///
    /// # Errors
    ///
    /// When the update fails (including an unknown `id`).
    pub async fn finish_export(
        db: &DatabaseConnection,
        id: Uuid,
        rows_total: u64,
        result_url: String,
    ) -> ModelResult<()> {
        let mut active = Self::load_active(db, id).await?;
        active.status = ActiveValue::set(JobStatus::Completed.as_str().to_string());
        active.rows_total = ActiveValue::set(Some(i64_of(rows_total)));
        active.rows_processed = ActiveValue::set(i64_of(rows_total));
        active.result_url = ActiveValue::set(Some(result_url));
        active.updated_at = ActiveValue::set(Utc::now().into());
        active.update(db).await?;
        Ok(())
    }

    /// Whether this job (and its artifacts) has passed its SEC-B4
    /// retention deadline. A job with no deadline (defensive: should not
    /// happen post-migration) never expires.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Utc::now() >= exp)
    }

    /// Load a job as an `ActiveModel` for a partial update, erroring if
    /// it is gone.
    async fn load_active(db: &DatabaseConnection, id: Uuid) -> ModelResult<ActiveModel> {
        let model = Self::find_by_id(db, id)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(model.into())
    }
}

#[cfg(test)]
mod tests {
    use super::NewBulkJob;
    use crate::bulk::BulkFormat;

    /// SEC-B9: the idempotency key is trimmed, and a blank/whitespace key
    /// is treated as absent (so it never dedupes against another blank).
    #[test]
    fn with_idempotency_key_trims_and_drops_blank() {
        let none = NewBulkJob::export(BulkFormat::Jsonl, serde_json::json!({}), None);
        assert_eq!(
            none.clone().with_idempotency_key(None).idempotency_key,
            None
        );
        assert_eq!(
            none.clone()
                .with_idempotency_key(Some("   ".to_string()))
                .idempotency_key,
            None,
            "a blank key is treated as absent"
        );
        assert_eq!(
            none.with_idempotency_key(Some("  abc-123  ".to_string()))
                .idempotency_key,
            Some("abc-123".to_string()),
            "a real key is trimmed"
        );
    }
}
