//! `bulk_jobs` model — durable lifecycle for one asynchronous bulk
//! operation.
//!
//! Replaces the process-local export registry that
//! [`crate::compliance::bulk`] used to keep, so a job survives a restart
//! and is visible to every replica. Follows the family contract in
//! [`agents/share/bulk-import-export.md`](../../../../agents/share/bulk-import-export.md)
//! §3.
//!
//! ## Status machine
//!
//! ```text
//! queued ──▶ running ──▶ completed
//!    │          │    └─▶ completed_with_errors  (import only — §7)
//!    │          └──────▶ failed
//!    └─────────────────▶ cancelled
//! ```
//!
//! `cancelled` is reachable from `queued` or `running` (the Bulk Data IG's
//! `DELETE` on the status endpoint). `completed_with_errors` is reachable
//! only from `running`, and only for an **import** job — an export never
//! has per-row failures of its own kind. A terminal status never changes
//! again, which is what lets the status endpoint be read without a lock.
//!
//! This table serves two consumers: the FHIR Bulk Data `$export`
//! operation (`entity = "care_pathway"`, `kind = "export"`,
//! `format = "ndjson"` — [`crate::workers::bulk_export`]) and the native
//! bulk import/export API (§13 T-10; `format = "jsonl" | "csv" | "tsv"` —
//! [`crate::bulk`]). They share one row shape and one table rather than
//! two, per the family contract; the `format` column is what
//! distinguishes an FHIR job from a native one when listing.

use chrono::{DateTime, FixedOffset, SubsecRound as _};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult, to_i64};

/// One bulk job row.
///
/// Timestamps are `DateTime<FixedOffset>`, the type the `SeaORM` entity
/// used, so anything that serializes a row keeps its exact wire form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Job id (application-assigned).
    pub id: Uuid,
    /// `import` | `export`.
    pub kind: String,
    /// The entity the job operates on.
    pub entity: String,
    /// Input/output format.
    pub format: String,
    /// Lifecycle status (see [`status`]).
    pub status: String,
    /// Request parameters.
    pub params: serde_json::Value,
    /// Total rows, once known.
    pub rows_total: Option<i64>,
    /// Rows processed so far.
    pub rows_processed: i64,
    /// Rows created (import).
    pub rows_created: i64,
    /// Rows upserted in place (import).
    pub rows_upserted: i64,
    /// Rows routed to the review queue (import).
    pub rows_to_review: i64,
    /// Rows that failed (import).
    pub rows_errored: i64,
    /// The submitting caller's `sub`.
    pub actor: Option<String>,
    /// Client-supplied key that makes a retried submit return the same job.
    pub idempotency_key: Option<String>,
    /// The uploaded input artifact.
    pub input_url: Option<String>,
    /// The output artifact.
    pub result_url: Option<String>,
    /// The per-row error report artifact.
    pub error_report_url: Option<String>,
    /// Why the job failed.
    pub error: Option<String>,
    /// Submission time.
    pub created_at: DateTime<FixedOffset>,
    /// Last transition time.
    pub updated_at: DateTime<FixedOffset>,
    /// When the job and its artifacts may be swept.
    pub expires_at: Option<DateTime<FixedOffset>>,
}

/// The current time, truncated to microseconds (what Postgres stores).
fn now() -> DateTime<FixedOffset> {
    chrono::Utc::now().trunc_subsecs(6).into()
}

/// A whole-row `RETURNING` that matched no row: `RowNotFound`, as
/// `SeaORM`'s `RecordNotUpdated` was.
fn gone(row: Option<Model>) -> ModelResult<Model> {
    row.ok_or(ModelError::Sqlx(sqlx::Error::RowNotFound))
}

/// Job kinds.
pub mod kind {
    /// An export job.
    pub const EXPORT: &str = "export";
    /// An import job (the native bulk API; not implemented yet).
    pub const IMPORT: &str = "import";
}

/// Job statuses.
pub mod status {
    /// Submitted, not yet picked up by a worker.
    pub const QUEUED: &str = "queued";
    /// A worker is materialising the job.
    pub const RUNNING: &str = "running";
    /// Finished; `result_url` references the output artifact.
    pub const COMPLETED: &str = "completed";
    /// Finished (import only): at least one row landed in the per-row
    /// error report (§7). `rows_created`/`rows_upserted`/`rows_to_review`
    /// still reflect every row that *did* commit — one bad row never
    /// aborts the rest of the load.
    pub const COMPLETED_WITH_ERRORS: &str = "completed_with_errors";
    /// Finished with a hard failure; `error` says why.
    pub const FAILED: &str = "failed";
    /// Cancelled by the client.
    pub const CANCELLED: &str = "cancelled";
}

/// Whether a status is terminal — no worker will move it again.
#[must_use]
pub fn is_terminal(s: &str) -> bool {
    matches!(
        s,
        status::COMPLETED | status::COMPLETED_WITH_ERRORS | status::FAILED | status::CANCELLED
    )
}

/// How long a completed **FHIR Bulk Data `$export`** job and its artifact
/// stay retrievable ([`submit`](Model::submit)).
///
/// The Bulk Data IG leaves retention to the server. Fifteen minutes
/// matches what the in-process registry offered, so adopting the durable
/// store does not silently extend how long clinical data sits in an
/// artifact.
pub const JOB_TTL_SECS: i64 = 900;

/// How long a **native bulk import/export** job and its artifacts stay
/// retrievable ([`submit_with_ttl`](Model::submit_with_ttl)).
///
/// A native bulk load is an operator-initiated, often large, one-off
/// action — the FHIR Bulk Data IG's fifteen-minute window (above) would
/// routinely expire before an operator downloads the result or the error
/// report. Seven days matches the family reference (organization/case
/// BLK-5).
pub const NATIVE_BULK_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// Saturating `u64` → `i64`, for the count columns.
fn i64_of(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

impl Model {
    /// Submit a **FHIR Bulk Data** job, or return the existing one for the
    /// same `idempotency_key`, on the [`JOB_TTL_SECS`] retention window.
    ///
    /// The key makes a retried submit return the same job rather than
    /// starting a second export of the same data (shared doc §3). Without
    /// a key every submit is a new job, which is the correct default for
    /// a `GET`-triggered `$export`.
    ///
    /// # Errors
    ///
    /// When the lookup or insert fails.
    pub async fn submit(
        db: &PgPool,
        entity: &str,
        job_kind: &str,
        format: &str,
        params: serde_json::Value,
        actor: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> ModelResult<Self> {
        let (job, _reused) = Self::submit_with_ttl(
            db,
            entity,
            job_kind,
            format,
            params,
            actor,
            idempotency_key,
            JOB_TTL_SECS,
        )
        .await?;
        Ok(job)
    }

    /// Submit a job on a caller-chosen retention window, reporting whether
    /// an existing job for the same `idempotency_key` was returned instead
    /// of a new insert ([`NATIVE_BULK_TTL_SECS`] — the native bulk
    /// import/export API, §13 T-10).
    ///
    /// The `UNIQUE (entity, kind, idempotency_key)` constraint backstops
    /// the check-then-insert race: on an insert failure with a key
    /// present, the existing row is re-fetched and returned as the race
    /// winner rather than propagating a spurious error.
    ///
    /// # Errors
    ///
    /// When the lookup fails, or the insert fails for a reason other than
    /// a losing idempotency race.
    #[allow(clippy::too_many_arguments)]
    pub async fn submit_with_ttl(
        db: &PgPool,
        entity: &str,
        job_kind: &str,
        format: &str,
        params: serde_json::Value,
        actor: Option<&str>,
        idempotency_key: Option<&str>,
        ttl_secs: i64,
    ) -> ModelResult<(Self, bool)> {
        if let Some(key) = idempotency_key
            && let Some(existing) = Self::find_by_idempotency_key(db, entity, job_kind, key).await?
        {
            return Ok((existing, true));
        }
        let now = now();
        let inserted = sqlx::query_as!(
            Self,
            r#"INSERT INTO bulk_jobs
                   (id, kind, entity, format, status, params, rows_total, rows_processed,
                    rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                    idempotency_key, input_url, result_url, error_report_url, error,
                    created_at, updated_at, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, NULL, 0, 0, 0, 0, 0, $7, $8,
                       NULL, NULL, NULL, NULL, $9, $9, $10)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         expires_at AS "expires_at: _",
                         id, kind, entity, format, status, params, rows_total, rows_processed,
                         rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                         idempotency_key, input_url, result_url, error_report_url, error"#,
            Uuid::new_v4(),
            job_kind,
            entity,
            format,
            status::QUEUED,
            params,
            actor,
            idempotency_key,
            now,
            now + chrono::Duration::seconds(ttl_secs),
        )
        .fetch_one(db)
        .await;
        match inserted {
            Ok(job) => Ok((job, false)),
            Err(insert_err) => {
                // Lost the race to a concurrent submit with the same key:
                // return the winner instead of the constraint violation.
                if let Some(key) = idempotency_key
                    && let Some(existing) =
                        Self::find_by_idempotency_key(db, entity, job_kind, key).await?
                {
                    return Ok((existing, true));
                }
                Err(insert_err.into())
            }
        }
    }

    /// Find an existing job for `entity` + `job_kind` bearing
    /// `idempotency_key`, or `None`. Matches the
    /// `UNIQUE (entity, kind, idempotency_key)` constraint so a retried
    /// submit resolves to the original job.
    ///
    /// # Errors
    ///
    /// When the query fails.
    async fn find_by_idempotency_key(
        db: &PgPool,
        entity: &str,
        job_kind: &str,
        key: &str,
    ) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                         expires_at AS "expires_at: _",
                         id, kind, entity, format, status, params, rows_total, rows_processed,
                         rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                         idempotency_key, input_url, result_url, error_report_url, error
               FROM bulk_jobs
               WHERE entity = $1 AND kind = $2 AND idempotency_key = $3
               LIMIT 1"#,
            entity,
            job_kind,
            key,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// Attach the uploaded input artifact reference to an import job.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn set_input_url(self, db: &PgPool, input_url: String) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE bulk_jobs SET input_url = $2, updated_at = $3 WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         expires_at AS "expires_at: _",
                         id, kind, entity, format, status, params, rows_total, rows_processed,
                         rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                         idempotency_key, input_url, result_url, error_report_url, error"#,
            self.id,
            input_url,
            now(),
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// Record the reconciled outcome of an **import** run (§7):
    /// `rows_total = rows_created + rows_upserted + rows_errored`
    /// (`rows_to_review` is a sub-count of `rows_created`, not a fourth
    /// exclusive bucket — see `crate::bulk::pipeline::ImportOutcome`).
    /// Status is [`status::COMPLETED`] when nothing errored, else
    /// [`status::COMPLETED_WITH_ERRORS`] — one bad row never aborts the
    /// rest of the load, so the job still finishes rather than failing.
    ///
    /// # Errors
    ///
    /// When the update fails.
    #[allow(clippy::too_many_arguments)]
    pub async fn finish_import(
        self,
        db: &PgPool,
        rows_total: u64,
        rows_created: u64,
        rows_upserted: u64,
        rows_to_review: u64,
        rows_errored: u64,
        error_report_url: Option<String>,
    ) -> ModelResult<Self> {
        let final_status = if rows_errored > 0 {
            status::COMPLETED_WITH_ERRORS
        } else {
            status::COMPLETED
        };
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE bulk_jobs
               SET status = $2, rows_total = $3, rows_processed = $3, rows_created = $4,
                   rows_upserted = $5, rows_to_review = $6, rows_errored = $7,
                   error_report_url = $8, updated_at = $9
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         expires_at AS "expires_at: _",
                         id, kind, entity, format, status, params, rows_total, rows_processed,
                         rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                         idempotency_key, input_url, result_url, error_report_url, error"#,
            self.id,
            final_status,
            i64_of(rows_total),
            i64_of(rows_created),
            i64_of(rows_upserted),
            i64_of(rows_to_review),
            i64_of(rows_errored),
            error_report_url,
            now(),
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// Fetch a job by id.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_id(db: &PgPool, id: Uuid) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                         expires_at AS "expires_at: _",
                         id, kind, entity, format, status, params, rows_total, rows_processed,
                         rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                         idempotency_key, input_url, result_url, error_report_url, error
               FROM bulk_jobs
               WHERE id = $1
               LIMIT 1"#,
            id,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// Claim a queued job for a worker, moving it to `running`.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn start(self, db: &PgPool) -> ModelResult<Self> {
        self.transition(db, status::RUNNING, None, None, None).await
    }

    /// Mark a job completed with its output artifact and row count.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn complete(self, db: &PgPool, result_url: &str, rows: i64) -> ModelResult<Self> {
        self.transition(db, status::COMPLETED, Some(result_url), None, Some(rows))
            .await
    }

    /// Mark a job failed, recording why.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn fail(self, db: &PgPool, reason: &str) -> ModelResult<Self> {
        self.transition(db, status::FAILED, None, Some(reason), None)
            .await
    }

    /// Cancel a job, clearing any output reference.
    ///
    /// The artifact itself is swept by retention; dropping the reference
    /// is what makes it unreachable, so a cancelled export stops serving
    /// clinical data immediately rather than at TTL.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn cancel(self, db: &PgPool) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE bulk_jobs SET status = $2, result_url = NULL, updated_at = $3
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         expires_at AS "expires_at: _",
                         id, kind, entity, format, status, params, rows_total, rows_processed,
                         rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                         idempotency_key, input_url, result_url, error_report_url, error"#,
            self.id,
            status::CANCELLED,
            now(),
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// The shared status transition.
    async fn transition(
        self,
        db: &PgPool,
        to: &str,
        result_url: Option<&str>,
        error: Option<&str>,
        rows: Option<i64>,
    ) -> ModelResult<Self> {
        // An absent value keeps the column as it is: `COALESCE` writes
        // back the stored value, which is what leaving the column out of
        // the `SeaORM` update did.
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE bulk_jobs
               SET status = $2,
                   result_url = COALESCE($3, result_url),
                   error = COALESCE($4, error),
                   rows_processed = COALESCE($5, rows_processed),
                   rows_total = COALESCE($5, rows_total),
                   updated_at = $6
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         expires_at AS "expires_at: _",
                         id, kind, entity, format, status, params, rows_total, rows_processed,
                         rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                         idempotency_key, input_url, result_url, error_report_url, error"#,
            self.id,
            to,
            result_url,
            error,
            rows,
            now(),
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// Whether the job has outlived its retention window.
    #[must_use]
    pub fn is_expired(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        self.expires_at.is_some_and(|e| e.to_utc() < now)
    }

    /// Recent jobs for this entity, newest first, optionally filtered by
    /// `kind` (`import`/`export`) and/or `status` (shared doc §4: `GET
    /// /api/care-pathways/bulk-jobs` "list (filter by `kind`/`status`)").
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(
        db: &PgPool,
        entity: &str,
        kind: Option<&str>,
        status: Option<&str>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                         expires_at AS "expires_at: _",
                         id, kind, entity, format, status, params, rows_total, rows_processed,
                         rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                         idempotency_key, input_url, result_url, error_report_url, error
               FROM bulk_jobs
               WHERE entity = $1
                 AND ($2::text IS NULL OR kind = $2)
                 AND ($3::text IS NULL OR status = $3)
               ORDER BY created_at DESC
               LIMIT $4"#,
            entity,
            kind,
            status,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}

/// Move every lost job to `failed`, returning the ids moved (see
/// [`crate::bulk::reconcile`] for what "lost" means). One statement, so a
/// job cannot finish between the check and the write. `job_worker` and
/// `export_worker` are the native bulk and FHIR `$export` workers' queue
/// class names, `grace_minutes` how long a job may sit with no queue row.
///
/// # Errors
///
/// When the statement fails, including when `axa_queue` does not exist.
///
/// Two names, because this table serves two consumers: a FHIR `$export`
/// row is run by `BulkExportWorker`, and matching only `BulkJobWorker`
/// would see every one of them as having no queue row, and fail them all.
pub async fn mark_lost(
    db: &PgPool,
    job_worker: &str,
    export_worker: &str,
    grace_minutes: i32,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        "UPDATE bulk_jobs b
         SET status = 'failed', error = 'lost by the job queue', updated_at = now()
         WHERE b.status IN ('queued', 'running')
           AND NOT EXISTS (
               SELECT 1 FROM axa_queue q
               WHERE q.name IN ($1, $2) AND q.args->>'job_id' = b.id::text AND q.status <> 'failed'
           )
           AND (
               EXISTS (
                   SELECT 1 FROM axa_queue q
                   WHERE q.name IN ($1, $2) AND q.args->>'job_id' = b.id::text AND q.status = 'failed'
               )
               OR b.updated_at < $3
           )
         RETURNING b.id",
        job_worker,
        export_worker,
        // `updated_at` is stamped by this process, so the grace is measured
        // on its clock too: Postgres `now()` would move it by the drift.
        chrono::Utc::now() - chrono::Duration::minutes(i64::from(grace_minutes)),
    )
    .fetch_all(db)
    .await
}

#[cfg(test)]
mod tests {
    use super::{is_terminal, status};

    /// Only the four finished states are terminal; a terminal status is
    /// what lets the status endpoint be read without taking a lock.
    #[test]
    fn terminal_states_are_exactly_the_finished_ones() {
        assert!(is_terminal(status::COMPLETED));
        assert!(is_terminal(status::COMPLETED_WITH_ERRORS));
        assert!(is_terminal(status::FAILED));
        assert!(is_terminal(status::CANCELLED));
        assert!(!is_terminal(status::QUEUED));
        assert!(!is_terminal(status::RUNNING));
        assert!(!is_terminal("something-else"));
    }

    /// The status tokens are the wire contract, so pin their spelling.
    #[test]
    fn status_tokens_are_stable() {
        assert_eq!(
            [
                status::QUEUED,
                status::RUNNING,
                status::COMPLETED,
                status::COMPLETED_WITH_ERRORS,
                status::FAILED,
                status::CANCELLED
            ],
            [
                "queued",
                "running",
                "completed",
                "completed_with_errors",
                "failed",
                "cancelled"
            ]
        );
    }

    /// `i64_of` saturates rather than panicking/wrapping on an absurd
    /// count.
    #[test]
    fn i64_of_saturates() {
        assert_eq!(super::i64_of(5), 5);
        assert_eq!(super::i64_of(u64::MAX), i64::MAX);
    }
}
