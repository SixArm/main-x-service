//! `bulk_jobs` model — CRUD/status helpers over the async bulk
//! import/export job table (BLK-5; `agents/share/bulk-import-export.md`
//! §3).
//!
//! Thin data-access helpers: create/enqueue a job, load one by id or by
//! idempotency key, and update its status + per-row counts + artifact
//! references as the background worker ([`crate::bulk::worker`])
//! progresses. The bulk **logic** lives in [`crate::bulk::pipeline`];
//! this module only reads and writes the row.

use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult};
use crate::bulk::pipeline::ImportOutcome;
use crate::bulk::{BulkFormat, BulkKind, JobStatus};

/// One persisted bulk-job row: kind + format + status, per-row counts,
/// and the artifact references (input, result, error report). Mirrors
/// the `bulk_jobs` table.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Application-assigned primary key (the job id).
    pub id: Uuid,
    /// `import` or `export`.
    pub kind: String,
    /// The entity name (`organization`).
    pub entity: String,
    /// File format token (`jsonl` | `csv` | `tsv`; no Parquet).
    pub format: String,
    /// `queued` | `running` | `completed` | `completed_with_errors` | `failed`.
    pub status: String,
    /// Free-form job parameters (dry-run flag, export filter, masking
    /// profile, …).
    pub params: serde_json::Value,
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
    pub created_at: DateTime<FixedOffset>,
    /// Last-update timestamp.
    pub updated_at: DateTime<FixedOffset>,
    /// When the job row and its artifacts may be swept (SEC-B4).
    pub expires_at: Option<DateTime<FixedOffset>>,
}

/// SEC-B4 — the job/artifact expiry stamp: `created_at` plus
/// [`crate::bulk::BULK_ARTIFACT_TTL_SECS`].
fn expires_at_of(created_at: DateTime<FixedOffset>) -> DateTime<FixedOffset> {
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
    pub async fn create(db: &PgPool, job: NewBulkJob) -> ModelResult<Self> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO bulk_jobs
                   (id, kind, entity, format, status, params, rows_total,
                    rows_processed, rows_created, rows_upserted, rows_to_review, rows_errored,
                    actor, idempotency_key, input_url, result_url, error_report_url,
                    created_at, updated_at, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, NULL, 0, 0, 0, 0, 0, $7, $8, NULL, NULL, NULL,
                       $9, $9, $10)
               RETURNING id, kind, entity, format, status, params, rows_total,
                         rows_processed, rows_created, rows_upserted, rows_to_review,
                         rows_errored, actor, idempotency_key, input_url, result_url,
                         error_report_url, created_at AS "created_at: _",
                         updated_at AS "updated_at: _", expires_at AS "expires_at: _""#,
            Uuid::new_v4(),
            job.kind.as_str(),
            crate::streaming::ENTITY,
            job.format.as_str(),
            JobStatus::Queued.as_str(),
            job.params,
            job.actor,
            job.idempotency_key,
            now,
            expires_at_of(now),
        )
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    /// Load one bulk job by id, or `None` if absent.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_id(db: &PgPool, id: Uuid) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT id, kind, entity, format, status, params, rows_total,
                      rows_processed, rows_created, rows_upserted, rows_to_review,
                      rows_errored, actor, idempotency_key, input_url, result_url,
                      error_report_url, created_at AS "created_at: _",
                      updated_at AS "updated_at: _", expires_at AS "expires_at: _"
               FROM bulk_jobs
               WHERE id = $1"#,
            id,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
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
        db: &PgPool,
        kind: BulkKind,
        key: &str,
    ) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT id, kind, entity, format, status, params, rows_total,
                      rows_processed, rows_created, rows_upserted, rows_to_review,
                      rows_errored, actor, idempotency_key, input_url, result_url,
                      error_report_url, created_at AS "created_at: _",
                      updated_at AS "updated_at: _", expires_at AS "expires_at: _"
               FROM bulk_jobs
               WHERE entity = $1 AND kind = $2 AND idempotency_key = $3
               LIMIT 1"#,
            crate::streaming::ENTITY,
            kind.as_str(),
            key,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
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
        db: &PgPool,
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
    pub async fn list_recent(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT id, kind, entity, format, status, params, rows_total,
                      rows_processed, rows_created, rows_upserted, rows_to_review,
                      rows_errored, actor, idempotency_key, input_url, result_url,
                      error_report_url, created_at AS "created_at: _",
                      updated_at AS "updated_at: _", expires_at AS "expires_at: _"
               FROM bulk_jobs
               ORDER BY created_at DESC
               LIMIT $1"#,
            i64::try_from(limit).unwrap_or(i64::MAX),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Attach the uploaded input artifact reference to an import job.
    ///
    /// # Errors
    ///
    /// When the update fails, or [`ModelError::EntityNotFound`] for an
    /// unknown `id`.
    pub async fn set_input_url(db: &PgPool, id: Uuid, input_url: String) -> ModelResult<()> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let done = sqlx::query!(
            "UPDATE bulk_jobs SET input_url = $2, updated_at = $3 WHERE id = $1",
            id,
            input_url,
            now,
        )
        .execute(db)
        .await?;
        found(done.rows_affected())
    }

    /// Transition a job to a new status, stamping `updated_at`.
    ///
    /// # Errors
    ///
    /// When the update fails, or [`ModelError::EntityNotFound`] for an
    /// unknown `id`.
    pub async fn set_status(db: &PgPool, id: Uuid, status: JobStatus) -> ModelResult<()> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let done = sqlx::query!(
            "UPDATE bulk_jobs SET status = $2, updated_at = $3 WHERE id = $1",
            id,
            status.as_str(),
            now,
        )
        .execute(db)
        .await?;
        found(done.rows_affected())
    }

    /// Record the outcome of an import run: final status, per-row counts,
    /// and the error-report reference.
    ///
    /// # Errors
    ///
    /// When the update fails, or [`ModelError::EntityNotFound`] for an
    /// unknown `id`.
    pub async fn finish_import(
        db: &PgPool,
        id: Uuid,
        outcome: &ImportOutcome,
        error_report_url: Option<String>,
    ) -> ModelResult<()> {
        let status = if outcome.rows_errored > 0 {
            JobStatus::CompletedWithErrors
        } else {
            JobStatus::Completed
        };
        let now: DateTime<FixedOffset> = Utc::now().into();
        let done = sqlx::query!(
            "UPDATE bulk_jobs
             SET status = $2, rows_total = $3, rows_processed = $3, rows_created = $4,
                 rows_upserted = $5, rows_to_review = $6, rows_errored = $7,
                 error_report_url = $8, updated_at = $9
             WHERE id = $1",
            id,
            status.as_str(),
            i64_of(outcome.rows_total),
            i64_of(outcome.rows_created),
            i64_of(outcome.rows_upserted),
            i64_of(outcome.rows_to_review),
            i64_of(outcome.rows_errored),
            error_report_url,
            now,
        )
        .execute(db)
        .await?;
        found(done.rows_affected())
    }

    /// Record the outcome of an export run: `completed` status, row
    /// count, and the output reference.
    ///
    /// # Errors
    ///
    /// When the update fails, or [`ModelError::EntityNotFound`] for an
    /// unknown `id`.
    pub async fn finish_export(
        db: &PgPool,
        id: Uuid,
        rows_total: u64,
        result_url: String,
    ) -> ModelResult<()> {
        let now: DateTime<FixedOffset> = Utc::now().into();
        let done = sqlx::query!(
            "UPDATE bulk_jobs
             SET status = $2, rows_total = $3, rows_processed = $3, result_url = $4,
                 updated_at = $5
             WHERE id = $1",
            id,
            JobStatus::Completed.as_str(),
            i64_of(rows_total),
            result_url,
            now,
        )
        .execute(db)
        .await?;
        found(done.rows_affected())
    }

    /// Move every lost job to `failed`, returning the ids moved (see
    /// [`crate::bulk::reconcile`] for what "lost" means). One statement,
    /// so a job cannot finish between the check and the write.
    /// `worker_name` is the bulk worker's queue class name, `grace_minutes`
    /// how long a job may sit with no queue row.
    ///
    /// # Errors
    ///
    /// When the statement fails, including when `axa_queue` does not
    /// exist.
    pub async fn mark_lost(
        db: &PgPool,
        worker_name: &str,
        grace_minutes: i32,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        sqlx::query_scalar!(
            "UPDATE bulk_jobs b
             SET status = 'failed', updated_at = now()
             WHERE b.status IN ('queued', 'running')
               AND NOT EXISTS (
                   SELECT 1 FROM axa_queue q
                   WHERE q.name = $1 AND q.args->>'job_id' = b.id::text AND q.status <> 'failed'
               )
               AND (
                   EXISTS (
                       SELECT 1 FROM axa_queue q
                       WHERE q.name = $1 AND q.args->>'job_id' = b.id::text AND q.status = 'failed'
                   )
                   OR b.updated_at < now() - make_interval(mins => $2)
               )
             RETURNING b.id",
            worker_name,
            grace_minutes,
        )
        .fetch_all(db)
        .await
    }

    /// Whether this job (and its artifacts) has passed its SEC-B4
    /// retention deadline. A job with no deadline (defensive: should not
    /// happen post-migration) never expires.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at.is_some_and(|exp| Utc::now() >= exp)
    }
}

/// A partial update hit no row: the job is gone.
fn found(rows_affected: u64) -> ModelResult<()> {
    if rows_affected == 0 {
        Err(ModelError::EntityNotFound)
    } else {
        Ok(())
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
