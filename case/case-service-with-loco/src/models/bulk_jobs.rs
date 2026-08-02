//! `bulk_jobs` persistence helpers (`agents/share/bulk-import-export.md`
//! §3, BLK-5).
//!
//! Thin data-access helpers over the [`bulk_jobs`](super::_entities::bulk_jobs)
//! `SeaORM` entity: create/enqueue a job, load one by id, and update its
//! status + per-row counts + artifact references as the background worker
//! ([`crate::bulk::worker`]) progresses. The bulk **logic** lives in
//! [`crate::bulk`]; this module only reads and writes the row. Mirrors the
//! shape of the person service's `db::bulk_jobs` (the family reference
//! implementation), adapted to this crate's loco model-error conventions
//! and `chrono` timestamps.

use chrono::SubsecRound as _;
use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

use crate::bulk::pipeline::ImportOutcome;
use crate::bulk::{BulkFormat, BulkKind, JobStatus};

pub use super::_entities::bulk_jobs::{self, ActiveModel, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for super::_entities::bulk_jobs::ActiveModel {}

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
pub async fn create(db: &DatabaseConnection, job: NewBulkJob) -> ModelResult<Model> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().trunc_subsecs(6).into();
    // SEC-B4: stamp a retention deadline so a stale export of case data is
    // not indefinitely retrievable (enforced at the status handler).
    let expires_at = now + chrono::Duration::seconds(BULK_ARTIFACT_TTL_SECS);
    let model = bulk_jobs::ActiveModel {
        id: Set(Uuid::new_v4()),
        kind: Set(job.kind.as_str().to_string()),
        entity: Set(crate::auth::ENTITY.to_string()),
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
        input_url: Set(None),
        result_url: Set(None),
        error_report_url: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        expires_at: Set(Some(expires_at)),
    };
    Ok(model.insert(db).await?)
}

/// Load one bulk job by id, or `None` if absent.
///
/// # Errors
///
/// When the query fails.
pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> ModelResult<Option<Model>> {
    Ok(bulk_jobs::Entity::find_by_id(id).one(db).await?)
}

/// Find an existing job for this entity + `kind` bearing `idempotency_key`
/// (SEC-B9), or `None`. Matches the `UNIQUE (entity, kind, idempotency_key)`
/// constraint so a retried submit resolves to the original job.
///
/// # Errors
///
/// When the query fails.
pub async fn find_by_idempotency_key(
    db: &DatabaseConnection,
    kind: BulkKind,
    key: &str,
) -> ModelResult<Option<Model>> {
    Ok(bulk_jobs::Entity::find()
        .filter(bulk_jobs::Column::Entity.eq(crate::auth::ENTITY))
        .filter(bulk_jobs::Column::Kind.eq(kind.as_str()))
        .filter(bulk_jobs::Column::IdempotencyKey.eq(key))
        .one(db)
        .await?)
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
pub async fn create_or_get_idempotent(
    db: &DatabaseConnection,
    job: NewBulkJob,
) -> ModelResult<(Model, bool)> {
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
pub async fn list_recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Model>> {
    Ok(bulk_jobs::Entity::find()
        .filter(bulk_jobs::Column::Entity.eq(crate::auth::ENTITY))
        .order_by_desc(bulk_jobs::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?)
}

/// Attach the uploaded input artifact reference to an import job.
///
/// # Errors
///
/// When the update fails.
pub async fn set_input_url(
    db: &DatabaseConnection,
    id: Uuid,
    input_url: String,
) -> ModelResult<()> {
    let mut active: bulk_jobs::ActiveModel = load_active(db, id).await?;
    active.input_url = Set(Some(input_url));
    active.updated_at = Set(chrono::Utc::now().trunc_subsecs(6).into());
    active.update(db).await?;
    Ok(())
}

/// Transition a job to a new status, stamping `updated_at`.
///
/// # Errors
///
/// When the update fails.
pub async fn set_status(db: &DatabaseConnection, id: Uuid, status: JobStatus) -> ModelResult<()> {
    let mut active: bulk_jobs::ActiveModel = load_active(db, id).await?;
    active.status = Set(status.as_str().to_string());
    active.updated_at = Set(chrono::Utc::now().trunc_subsecs(6).into());
    active.update(db).await?;
    Ok(())
}

/// Record the outcome of an import run: final status, per-row counts, and
/// the error-report reference.
///
/// # Errors
///
/// When the update fails.
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
    let mut active: bulk_jobs::ActiveModel = load_active(db, id).await?;
    active.status = Set(status.as_str().to_string());
    active.rows_total = Set(Some(i64_of(outcome.rows_total)));
    active.rows_processed = Set(i64_of(outcome.rows_total));
    active.rows_created = Set(i64_of(outcome.rows_created));
    active.rows_upserted = Set(i64_of(outcome.rows_upserted));
    active.rows_to_review = Set(i64_of(outcome.rows_to_review));
    active.rows_errored = Set(i64_of(outcome.rows_errored));
    active.error_report_url = Set(error_report_url);
    active.updated_at = Set(chrono::Utc::now().trunc_subsecs(6).into());
    active.update(db).await?;
    Ok(())
}

/// Record the outcome of an export run: `completed` status, row count,
/// and the output reference.
///
/// # Errors
///
/// When the update fails.
pub async fn finish_export(
    db: &DatabaseConnection,
    id: Uuid,
    rows_total: u64,
    result_url: String,
) -> ModelResult<()> {
    let mut active: bulk_jobs::ActiveModel = load_active(db, id).await?;
    active.status = Set(JobStatus::Completed.as_str().to_string());
    active.rows_total = Set(Some(i64_of(rows_total)));
    active.rows_processed = Set(i64_of(rows_total));
    active.result_url = Set(Some(result_url));
    active.updated_at = Set(chrono::Utc::now().trunc_subsecs(6).into());
    active.update(db).await?;
    Ok(())
}

/// Mark a job `failed` (a whole-job failure, e.g. an unreadable artifact).
///
/// # Errors
///
/// When the update fails.
pub async fn fail(db: &DatabaseConnection, id: Uuid) -> ModelResult<()> {
    set_status(db, id, JobStatus::Failed).await
}

/// Load a job as an `ActiveModel` for update, erroring if it is gone.
async fn load_active(db: &DatabaseConnection, id: Uuid) -> ModelResult<bulk_jobs::ActiveModel> {
    let model = find_by_id(db, id)
        .await?
        .ok_or(ModelError::EntityNotFound)?;
    Ok(model.into())
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
    use sea_orm::DatabaseConnection;
    use uuid::Uuid;

    async fn connect() -> DatabaseConnection {
        use migration::MigratorTrait as _;
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        let db = sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL");
        // Unlike the request-level suites under `tests/requests/`, which
        // boot the loco `App` (auto-migrating), these tests connect
        // directly — so migrations are applied here explicitly. Idempotent:
        // `sea-orm-migration` tracks applied migrations and skips them on
        // a later call, so running this per test function is safe.
        migration::Migrator::up(&db, None)
            .await
            .expect("run migrations");
        db
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
