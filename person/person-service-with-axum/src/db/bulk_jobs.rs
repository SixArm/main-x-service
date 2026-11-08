//! `bulk_jobs` persistence helpers (`agents/share/bulk-import-export.md`
//! §3).
//!
//! Thin data-access helpers over the [`bulk_jobs`](crate::db::models::bulk_jobs)
//! table: create/enqueue a job, load one by id, and update its
//! status + per-row counts + artifact references as the background worker
//! progresses. The bulk **logic** lives in [`crate::bulk`]; this module
//! only reads and writes the row.

use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::convert::offset_to_ts;
use super::to_i64;
use crate::Result;
use crate::bulk::pipeline::ImportOutcome;
use crate::bulk::{BulkFormat, BulkKind, JobStatus};
use crate::db::models::{BulkJobRaw, bulk_jobs};

pub use bulk_jobs::Model;

/// The fields needed to enqueue a new bulk job.
#[derive(Debug, Clone)]
pub struct NewBulkJob {
    /// Import or export.
    pub kind: BulkKind,
    /// File format (`jsonl`, `csv`, or `parquet` — export-only).
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
    /// An **import** job in `format` with the given params and actor. The
    /// input artifact is attached afterwards via [`set_input_url`].
    #[must_use]
    pub fn import(format: BulkFormat, params: serde_json::Value, actor: Option<String>) -> Self {
        Self {
            kind: BulkKind::Import,
            format,
            params,
            actor,
            idempotency_key: None,
            input_url: None,
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
            input_url: None,
        }
    }

    /// Attach a client-supplied idempotency key (SEC-B9), trimmed; a blank
    /// key is treated as absent. A retried submit carrying the same key
    /// dedupes to the same job (`create_or_get_idempotent`).
    #[must_use]
    pub fn with_idempotency_key(mut self, key: Option<String>) -> Self {
        self.idempotency_key = key.map(|k| k.trim().to_string()).filter(|k| !k.is_empty());
        self
    }
}

/// Insert a new `queued` bulk job and return its persisted row.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the insert fails.
pub async fn create(db: &PgPool, job: NewBulkJob) -> Result<Model> {
    let now = OffsetDateTime::now_utc();
    // SEC-B4: stamp a retention deadline so a stale export of personal data
    // is not indefinitely retrievable (enforced at the status handler).
    let expires_at = now + time::Duration::seconds(crate::bulk::BULK_ARTIFACT_TTL_SECS);
    let row = sqlx::query_as!(
        BulkJobRaw,
        "INSERT INTO bulk_jobs
             (id, kind, entity, format, status, params, rows_total, rows_processed,
              rows_created, rows_upserted, rows_to_review, rows_errored, actor,
              idempotency_key, input_url, result_url, error_report_url, created_at,
              updated_at, expires_at, artifact_deleted_at)
         VALUES ($1, $2, $3, $4, $5, $6, NULL, 0, 0, 0, 0, 0, $7, $8, $9, NULL, NULL, $10,
                 $10, $11, NULL)
         RETURNING id, kind, entity, format, status, params, rows_total, rows_processed,
                rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                idempotency_key, input_url, result_url, error_report_url, created_at,
                updated_at, expires_at, artifact_deleted_at",
        Uuid::new_v4(),
        job.kind.as_str(),
        crate::streaming::ENTITY,
        job.format.as_str(),
        JobStatus::Queued.as_str(),
        job.params,
        job.actor,
        job.idempotency_key,
        job.input_url,
        offset_to_ts(now),
        offset_to_ts(expires_at),
    )
    .fetch_one(db)
    .await?;
    Ok(row.into())
}

/// Load one bulk job by id, or `None` if absent.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn find_by_id(db: &PgPool, id: Uuid) -> Result<Option<Model>> {
    let row = sqlx::query_as!(
        BulkJobRaw,
        "SELECT id, kind, entity, format, status, params, rows_total, rows_processed,
                rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                idempotency_key, input_url, result_url, error_report_url, created_at,
                updated_at, expires_at, artifact_deleted_at
         FROM bulk_jobs
         WHERE id = $1
         LIMIT 1",
        id,
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(Into::into))
}

/// Find an existing job for this entity + `kind` bearing `idempotency_key`
/// (SEC-B9), or `None`. Matches the `UNIQUE (entity, kind, idempotency_key)`
/// constraint so a retried submit resolves to the original job.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn find_by_idempotency_key(
    db: &PgPool,
    kind: BulkKind,
    key: &str,
) -> Result<Option<Model>> {
    let row = sqlx::query_as!(
        BulkJobRaw,
        "SELECT id, kind, entity, format, status, params, rows_total, rows_processed,
                rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                idempotency_key, input_url, result_url, error_report_url, created_at,
                updated_at, expires_at, artifact_deleted_at
         FROM bulk_jobs
         WHERE entity = $1 AND kind = $2 AND idempotency_key = $3
         LIMIT 1",
        crate::streaming::ENTITY,
        kind.as_str(),
        key,
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(Into::into))
}

/// Create a job **idempotently** (SEC-B9): if it carries an idempotency key
/// that already names a job (this entity + kind), return that existing job
/// (`reused = true`) instead of inserting a duplicate — so a retried submit
/// neither re-runs the work nor creates a second row. A key-less job always
/// inserts. The `UNIQUE (entity, kind, idempotency_key)` constraint backstops
/// the check-then-insert race: on a unique violation the existing row is
/// re-fetched and returned.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query/insert fails for a reason
/// other than a losing idempotency race.
pub async fn create_or_get_idempotent(db: &PgPool, job: NewBulkJob) -> Result<(Model, bool)> {
    if let Some(key) = job.idempotency_key.clone() {
        if let Some(existing) = find_by_idempotency_key(db, job.kind, &key).await? {
            return Ok((existing, true));
        }
        let kind = job.kind;
        return match create(db, job).await {
            Ok(model) => Ok((model, false)),
            Err(insert_err) => {
                // Possible unique-violation race: a concurrent submit with
                // the same key won. Re-check and return the winner.
                if let Some(existing) = find_by_idempotency_key(db, kind, &key).await? {
                    Ok((existing, true))
                } else {
                    Err(insert_err)
                }
            }
        };
    }
    Ok((create(db, job).await?, false))
}

/// List the most recent bulk jobs (newest first), capped at `limit`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn list_recent(db: &PgPool, limit: u64) -> Result<Vec<Model>> {
    let rows = sqlx::query_as!(
        BulkJobRaw,
        "SELECT id, kind, entity, format, status, params, rows_total, rows_processed,
                rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                idempotency_key, input_url, result_url, error_report_url, created_at,
                updated_at, expires_at, artifact_deleted_at
         FROM bulk_jobs
         ORDER BY created_at DESC
         LIMIT $1",
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Attach the uploaded input artifact reference to an import job.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn set_input_url(db: &PgPool, id: Uuid, input_url: String) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE bulk_jobs SET input_url = $2, updated_at = $3 WHERE id = $1",
        id,
        input_url,
        now(),
    )
    .execute(db)
    .await?;
    found(id, done.rows_affected())
}

/// Transition a job to a new status, stamping `updated_at`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn set_status(db: &PgPool, id: Uuid, status: JobStatus) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE bulk_jobs SET status = $2, updated_at = $3 WHERE id = $1",
        id,
        status.as_str(),
        now(),
    )
    .execute(db)
    .await?;
    found(id, done.rows_affected())
}

/// Record the outcome of an import run: final status, per-row counts, and
/// the error-report reference.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn finish_import(
    db: &PgPool,
    id: Uuid,
    outcome: &ImportOutcome,
    error_report_url: Option<String>,
) -> Result<()> {
    let status = if outcome.rows_errored > 0 {
        JobStatus::CompletedWithErrors
    } else {
        JobStatus::Completed
    };
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
        now(),
    )
    .execute(db)
    .await?;
    found(id, done.rows_affected())
}

/// Record the outcome of an export run: `completed` status, row count,
/// and the output reference.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn finish_export(
    db: &PgPool,
    id: Uuid,
    rows_total: u64,
    result_url: String,
) -> Result<()> {
    let done = sqlx::query!(
        "UPDATE bulk_jobs
         SET status = $2, rows_total = $3, rows_processed = $3, result_url = $4,
             updated_at = $5
         WHERE id = $1",
        id,
        JobStatus::Completed.as_str(),
        i64_of(rows_total),
        result_url,
        now(),
    )
    .execute(db)
    .await?;
    found(id, done.rows_affected())
}

/// Stamp `artifact_deleted_at = now` on a job (SEC-B4 follow-up): the
/// periodic sweep ([`crate::bulk::sweep`]) has physically removed this
/// job's artifacts from the store, so it must never be selected by
/// [`list_artifact_sweep_candidates`] again.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn mark_artifact_deleted(db: &PgPool, id: Uuid) -> Result<()> {
    let now = now();
    let done = sqlx::query!(
        "UPDATE bulk_jobs SET artifact_deleted_at = $2, updated_at = $2 WHERE id = $1",
        id,
        now,
    )
    .execute(db)
    .await?;
    found(id, done.rows_affected())
}

/// Jobs eligible for the physical-artifact sweep (SEC-B4 follow-up): past
/// their retention deadline (`expires_at <= now`) and not yet swept
/// (`artifact_deleted_at IS NULL`), oldest deadline first, capped at
/// `limit`. A job with no deadline at all (a legacy row predating
/// `expires_at`) is never selected — mirrors
/// [`crate::bulk::handlers`]'s `artifact_expired`, which likewise never
/// expires a deadline-less job.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn list_artifact_sweep_candidates(
    db: &PgPool,
    now: OffsetDateTime,
    limit: u64,
) -> Result<Vec<Model>> {
    let rows = sqlx::query_as!(
        BulkJobRaw,
        "SELECT id, kind, entity, format, status, params, rows_total, rows_processed,
                rows_created, rows_upserted, rows_to_review, rows_errored, actor,
                idempotency_key, input_url, result_url, error_report_url, created_at,
                updated_at, expires_at, artifact_deleted_at
         FROM bulk_jobs
         WHERE expires_at IS NOT NULL AND expires_at <= $1 AND artifact_deleted_at IS NULL
         ORDER BY expires_at ASC
         LIMIT $2",
        offset_to_ts(now),
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Move every lost job to `failed` in one statement, so a job cannot
/// finish between the check and the write ([`crate::bulk::reconcile`]).
/// `worker` is the queue worker's class name; a job is lost when no live
/// queue row for it remains and either one has failed or it has none and
/// has not changed for `grace_minutes`. Returns the ids moved.
///
/// # Errors
///
/// When the statement fails, including when `axa_queue` does not exist.
pub async fn mark_lost(db: &PgPool, worker: &str, grace_minutes: i32) -> Result<Vec<Uuid>> {
    let moved = sqlx::query_scalar!(
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
        worker,
        grace_minutes,
    )
    .fetch_all(db)
    .await?;
    Ok(moved)
}

/// SEC-B3: take a transaction-scoped advisory lock on a bulk import row's
/// stable key, held to the end of `conn`'s transaction, so two importers of
/// the same key serialise ([`crate::bulk::pipeline`]).
///
/// # Errors
///
/// When the lock query fails.
pub async fn lock_stable_key(conn: &mut PgConnection, key: &str) -> Result<()> {
    sqlx::query!("SELECT pg_advisory_xact_lock(hashtext($1)::bigint)", key)
        .execute(conn)
        .await?;
    Ok(())
}

/// The time an update stamps on `updated_at`, as the database takes it.
fn now() -> chrono::DateTime<chrono::Utc> {
    offset_to_ts(OffsetDateTime::now_utc())
}

/// An update that matched no row is the "not found" error the previous
/// load-then-update reported.
fn found(id: Uuid, rows: u64) -> Result<()> {
    if rows == 0 {
        return Err(crate::Error::Internal(format!("bulk job {id} not found")));
    }
    Ok(())
}

/// Saturating `u64` → `i64` for count columns.
fn i64_of(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::NewBulkJob;
    use crate::bulk::BulkFormat;

    /// SEC-B9: the idempotency key is trimmed, and a blank/whitespace key is
    /// treated as absent (so it never dedupes against another blank).
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

/// DB-gated (`#[ignore]`) tests for the bulk-jobs helpers. They need a
/// migrated `PostgreSQL` via `DATABASE_URL`; a bare `cargo test` skips them
/// but they MUST compile under `cargo test --no-run`.
#[cfg(test)]
mod db_tests {
    use super::{NewBulkJob, create, create_or_get_idempotent, find_by_id};
    use crate::bulk::{BulkFormat, BulkKind};
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn connect() -> PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        PgPool::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// SEC-B9: re-submitting with the same idempotency key returns the same
    /// job (reused) and creates no second row; a different key is a new job.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn idempotent_resubmit_returns_the_same_job() {
        let db = connect().await;
        let key = format!("idem-{}", Uuid::new_v4());

        let first = NewBulkJob::export(
            BulkFormat::Jsonl,
            serde_json::json!({}),
            Some("actor".to_string()),
        )
        .with_idempotency_key(Some(key.clone()));
        let (job1, reused1) = create_or_get_idempotent(&db, first).await.unwrap();
        assert!(!reused1, "first submit creates the job");

        // Same key ⇒ same job, marked reused, no new row.
        let retry = NewBulkJob::export(
            BulkFormat::Jsonl,
            serde_json::json!({}),
            Some("actor".to_string()),
        )
        .with_idempotency_key(Some(key.clone()));
        let (job2, reused2) = create_or_get_idempotent(&db, retry).await.unwrap();
        assert!(reused2, "retried submit is deduped");
        assert_eq!(job1.id, job2.id, "same job id returned");

        // A different key is a distinct job.
        let other = NewBulkJob::export(
            BulkFormat::Jsonl,
            serde_json::json!({}),
            Some("actor".to_string()),
        )
        .with_idempotency_key(Some(format!("idem-{}", Uuid::new_v4())));
        let (job3, reused3) = create_or_get_idempotent(&db, other).await.unwrap();
        assert!(!reused3);
        assert_ne!(job3.id, job1.id);

        // Ground truth: both deduped submits point at the one persisted row.
        assert!(find_by_id(&db, job1.id).await.unwrap().is_some());
    }

    /// A key-less submit always creates a fresh job (never deduped).
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn keyless_submit_is_never_deduped() {
        let db = connect().await;
        let (a, _) = create_or_get_idempotent(
            &db,
            NewBulkJob::import(
                BulkFormat::Jsonl,
                serde_json::json!({ "dry_run": true }),
                None,
            ),
        )
        .await
        .unwrap();
        let b = create(
            &db,
            NewBulkJob::import(
                BulkFormat::Jsonl,
                serde_json::json!({ "dry_run": true }),
                None,
            ),
        )
        .await
        .unwrap();
        assert_eq!(a.kind, BulkKind::Import.as_str());
        assert_ne!(a.id, b.id, "key-less jobs are always distinct");
    }
}
