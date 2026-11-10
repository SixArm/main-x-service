//! `bulk_jobs` persistence helpers (`agents/share/bulk-import-export.md`
//! §3, BLK-5).
//!
//! Thin data-access helpers over the `bulk_jobs` table: create/enqueue a
//! job, load one by id, and update its
//! status + per-row counts + artifact references as the background worker
//! ([`crate::bulk::worker`]) progresses. The bulk **logic** lives in
//! [`crate::bulk`]; this module only reads and writes the row. Mirrors the
//! shape of the person service's `db::bulk_jobs` (the family reference
//! implementation), adapted to this crate's model-error conventions and
//! `chrono` timestamps.
//!
//! A partial update first looked the row up under `SeaORM`, so an
//! unknown id is still [`super::ModelError::EntityNotFound`]; each update
//! is now one statement that checks it hit a row.

use chrono::{DateTime, FixedOffset, SubsecRound as _, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelResult, found, to_i64};
use crate::bulk::pipeline::ImportOutcome;
use crate::bulk::{BulkFormat, BulkKind, JobStatus};

/// One bulk job: its kind, status, counters, and artifact references.
/// Mirrors the `bulk_jobs` table.
///
/// Timestamps are `DateTime<FixedOffset>`, the type the `SeaORM` entity
/// used, so anything that serializes a row keeps its exact wire form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Job id.
    pub id: Uuid,
    /// `import` | `export`.
    pub kind: String,
    /// The entity the job operates on (`case`).
    pub entity: String,
    /// Input/output format (`jsonl` | `csv`).
    pub format: String,
    /// `queued` | `running` | `completed` | `completed_with_errors` | `failed`.
    pub status: String,
    /// Request parameters, so a job stays interpretable after the fact.
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
    /// The submitting caller's `sub`, when a verified token was presented.
    pub actor: Option<String>,
    /// Client-supplied key making a retried submit return the same job.
    pub idempotency_key: Option<String>,
    /// Artifact reference for the uploaded source file.
    pub input_url: Option<String>,
    /// Reference to the output artifact, set when the job completes.
    pub result_url: Option<String>,
    /// Artifact reference for the downloadable per-row error report.
    pub error_report_url: Option<String>,
    /// When the job was submitted.
    pub created_at: DateTime<FixedOffset>,
    /// When the job last changed state.
    pub updated_at: DateTime<FixedOffset>,
    /// When the row and its artifacts may be swept (SEC-B4).
    pub expires_at: Option<DateTime<FixedOffset>>,
}

/// The current instant, truncated to microseconds (what Postgres keeps).
fn now() -> DateTime<FixedOffset> {
    Utc::now().trunc_subsecs(6).into()
}

/// SEC-B4 — how long a bulk job and its artifacts stay retrievable, so a
/// stale export of case data (personal data) is not indefinitely
/// downloadable. Matches the person reference implementation's window.
pub const BULK_ARTIFACT_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// The fields needed to enqueue a new bulk job.
#[derive(Debug, Clone)]
pub struct NewBulkJob {
    /// Import or export.
    pub kind: BulkKind,
    /// File format (`jsonl` or `csv`).
    pub format: BulkFormat,
    /// Free-form parameters (dry-run flag, export filter, …).
    pub params: serde_json::Value,
    /// Acting user pid (bearer `sub`), if any.
    pub actor: Option<String>,
    /// Client-supplied idempotency key, if any.
    pub idempotency_key: Option<String>,
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
    /// dedupes to the same job ([`create_or_get_idempotent`]).
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
/// When the insert fails.
pub async fn create(db: &PgPool, job: NewBulkJob) -> ModelResult<Model> {
    let now = now();
    // SEC-B4: stamp a retention deadline so a stale export of case data is
    // not indefinitely retrievable (enforced at the status handler).
    let expires_at = now + chrono::Duration::seconds(BULK_ARTIFACT_TTL_SECS);
    let row = sqlx::query_as!(
        Model,
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
        crate::auth::ENTITY,
        job.format.as_str(),
        JobStatus::Queued.as_str(),
        job.params,
        job.actor,
        job.idempotency_key,
        now,
        expires_at,
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
pub async fn find_by_id(db: &PgPool, id: Uuid) -> ModelResult<Option<Model>> {
    let row = sqlx::query_as!(
        Model,
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

/// Find an existing job for this entity + `kind` bearing `idempotency_key`
/// (SEC-B9), or `None`. Matches the `UNIQUE (entity, kind, idempotency_key)`
/// constraint so a retried submit resolves to the original job.
///
/// # Errors
///
/// When the query fails.
pub async fn find_by_idempotency_key(
    db: &PgPool,
    kind: BulkKind,
    key: &str,
) -> ModelResult<Option<Model>> {
    let row = sqlx::query_as!(
        Model,
        r#"SELECT id, kind, entity, format, status, params, rows_total,
                  rows_processed, rows_created, rows_upserted, rows_to_review,
                  rows_errored, actor, idempotency_key, input_url, result_url,
                  error_report_url, created_at AS "created_at: _",
                  updated_at AS "updated_at: _", expires_at AS "expires_at: _"
           FROM bulk_jobs
           WHERE entity = $1 AND kind = $2 AND idempotency_key = $3
           LIMIT 1"#,
        crate::auth::ENTITY,
        kind.as_str(),
        key,
    )
    .fetch_optional(db)
    .await?;
    Ok(row)
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
/// When the query/insert fails for a reason other than a losing
/// idempotency race.
pub async fn create_or_get_idempotent(db: &PgPool, job: NewBulkJob) -> ModelResult<(Model, bool)> {
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

/// List the most recent bulk jobs for this entity (newest first), capped
/// at `limit`.
///
/// # Errors
///
/// When the query fails.
pub async fn list_recent(db: &PgPool, limit: u64) -> ModelResult<Vec<Model>> {
    let rows = sqlx::query_as!(
        Model,
        r#"SELECT id, kind, entity, format, status, params, rows_total,
                  rows_processed, rows_created, rows_upserted, rows_to_review,
                  rows_errored, actor, idempotency_key, input_url, result_url,
                  error_report_url, created_at AS "created_at: _",
                  updated_at AS "updated_at: _", expires_at AS "expires_at: _"
           FROM bulk_jobs
           WHERE entity = $1
           ORDER BY created_at DESC
           LIMIT $2"#,
        crate::auth::ENTITY,
        to_i64(limit),
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// Attach the uploaded input artifact reference to an import job.
///
/// # Errors
///
/// When the update fails.
pub async fn set_input_url(db: &PgPool, id: Uuid, input_url: String) -> ModelResult<()> {
    let done = sqlx::query!(
        "UPDATE bulk_jobs SET input_url = $2, updated_at = $3 WHERE id = $1",
        id,
        input_url,
        now(),
    )
    .execute(db)
    .await?;
    found(done.rows_affected())
}

/// Transition a job to a new status, stamping `updated_at`.
///
/// # Errors
///
/// When the update fails, or [`super::ModelError::EntityNotFound`] for an
/// unknown `id`.
pub async fn set_status(db: &PgPool, id: Uuid, status: JobStatus) -> ModelResult<()> {
    let done = sqlx::query!(
        "UPDATE bulk_jobs SET status = $2, updated_at = $3 WHERE id = $1",
        id,
        status.as_str(),
        now(),
    )
    .execute(db)
    .await?;
    found(done.rows_affected())
}

/// Record the outcome of an import run: final status, per-row counts, and
/// the error-report reference.
///
/// # Errors
///
/// When the update fails, or [`super::ModelError::EntityNotFound`] for an
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
    found(done.rows_affected())
}

/// Record the outcome of an export run: `completed` status, row count,
/// and the output reference.
///
/// # Errors
///
/// When the update fails, or [`super::ModelError::EntityNotFound`] for an
/// unknown `id`.
pub async fn finish_export(
    db: &PgPool,
    id: Uuid,
    rows_total: u64,
    result_url: String,
) -> ModelResult<()> {
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
    found(done.rows_affected())
}

/// Mark a job `failed` (a whole-job failure, e.g. an unreadable artifact).
///
/// # Errors
///
/// When the update fails.
pub async fn fail(db: &PgPool, id: Uuid) -> ModelResult<()> {
    set_status(db, id, JobStatus::Failed).await
}

/// Move every lost job to `failed`, returning the ids moved (see
/// [`crate::bulk::reconcile`] for what "lost" means). One statement, so a
/// job cannot finish between the check and the write. `worker_name` is
/// the bulk worker's queue class name, `grace_minutes` how long a job may
/// sit with no queue row.
///
/// # Errors
///
/// When the statement fails, including when `axa_queue` does not exist.
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
               OR b.updated_at < $2
           )
         RETURNING b.id",
        worker_name,
        // `updated_at` is stamped by this process, so the grace is measured
        // on its clock too: Postgres `now()` would move it by the drift.
        chrono::Utc::now() - chrono::Duration::minutes(i64::from(grace_minutes)),
    )
    .fetch_all(db)
    .await
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
/// migrated `PostgreSQL` via `DB_URL`; a bare `cargo test` skips them
/// but they MUST compile under `cargo test --no-run`.
#[cfg(test)]
mod db_tests {
    use super::{NewBulkJob, create, create_or_get_idempotent, find_by_id};
    use crate::bulk::{BulkFormat, BulkKind};
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn connect() -> PgPool {
        let url = std::env::var("DB_URL").expect("DB_URL must be set for DB tests");
        let db = PgPool::connect(&url).await.expect("connect to DB_URL");
        // Unlike the request-level suites under `tests/requests/`, which
        // boot the axa `App` (auto-migrating), these tests connect
        // directly — so migrations are applied here explicitly. Idempotent:
        // sqlx tracks applied migrations and skips them on a later call, so
        // running this per test function is safe.
        crate::models::MIGRATOR
            .run(&db)
            .await
            .expect("run migrations");
        db
    }

    /// SEC-B9: re-submitting with the same idempotency key returns the same
    /// job (reused) and creates no second row; a different key is a new job.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
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

        let retry = NewBulkJob::export(
            BulkFormat::Jsonl,
            serde_json::json!({}),
            Some("actor".to_string()),
        )
        .with_idempotency_key(Some(key.clone()));
        let (job2, reused2) = create_or_get_idempotent(&db, retry).await.unwrap();
        assert!(reused2, "retried submit is deduped");
        assert_eq!(job1.id, job2.id, "same job id returned");

        let other = NewBulkJob::export(
            BulkFormat::Jsonl,
            serde_json::json!({}),
            Some("actor".to_string()),
        )
        .with_idempotency_key(Some(format!("idem-{}", Uuid::new_v4())));
        let (job3, reused3) = create_or_get_idempotent(&db, other).await.unwrap();
        assert!(!reused3);
        assert_ne!(job3.id, job1.id);

        assert!(find_by_id(&db, job1.id).await.unwrap().is_some());
    }

    /// A key-less submit always creates a fresh job (never deduped).
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DB_URL"]
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
