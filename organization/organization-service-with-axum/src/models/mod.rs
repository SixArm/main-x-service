//! Persistence: the table row types and **every SQL statement this
//! service runs** (spec/tech-stack §5.2).
//!
//! Each module owns one table: a plain row struct, and the helpers the
//! controllers, the relay, the bulk worker and the tasks call. Every
//! statement is a `sqlx::query!` / `query_as!` / `query_scalar!` literal,
//! checked at compile time against the committed `.sqlx/` data, so a
//! reviewer reads the whole of the service's SQL in this directory. The
//! schema itself is the plain SQL under `migrations/`, run by
//! [`MIGRATOR`].
//!
//! A helper that writes takes an [`axa::sqlx::Db`], so the caller passes
//! either the pool or its own open transaction (`&mut *txn`). That is how
//! an entity row, its `event_outbox` row and its audit row commit in one
//! transaction under the `outbox` transport.

use std::fmt;

/// Audit-trail recording and query helpers over `audit_logs`.
pub mod audit_logs;
/// CRUD/status helpers over `bulk_jobs` (BLK-5 async bulk import/export),
/// plus the lost-job reconciliation statement.
pub mod bulk_jobs;
/// Durable event-bus Phase-2 transactional-outbox write + relay helpers
/// over `event_outbox` (`OutboxInsert::from_envelope`, `insert_on`,
/// `recent`, relay poll/ack, retention purge).
pub mod event_outbox;
/// Merge-history recording and query helpers over `merge_records`.
pub mod merge_records;
/// CRUD/search helpers over `organizations` (stored payload).
pub mod organizations;
/// The batch-deduplication review queue (normalized-pair upsert / list /
/// first-writer-wins decide), mirroring the person/worker/place/thing
/// registries.
pub mod review_queue;

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `organization-service db migrate | down | reset | status` and the
/// boot auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A model-layer failure.
///
/// The variant names are stable, so a caller's match does not move, and each
/// converts into an [`axa::Error`] with a fixed status: a missing row is
/// `404`, a database error is axa's own mapping (a violated unique constraint
/// is `409`, anything else `500`), and any other failure is `500`.
#[derive(Debug)]
pub enum ModelError {
    /// The row does not exist (or is soft-deleted).
    EntityNotFound,
    /// A database failure.
    Sqlx(sqlx::Error),
    /// Any other failure (a malformed pid, a payload that does not parse).
    Any(Box<dyn std::error::Error + Send + Sync>),
}

/// A model-layer result.
pub type ModelResult<T> = std::result::Result<T, ModelError>;

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityNotFound => f.write_str("Entity not found"),
            Self::Sqlx(e) => e.fmt(f),
            Self::Any(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ModelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EntityNotFound => None,
            Self::Sqlx(e) => Some(e),
            Self::Any(e) => Some(e.as_ref()),
        }
    }
}

impl From<sqlx::Error> for ModelError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}

impl From<ModelError> for axa::Error {
    fn from(e: ModelError) -> Self {
        match e {
            ModelError::EntityNotFound => Self::NotFound,
            ModelError::Sqlx(e) => Self::Sqlx(e),
            ModelError::Any(e) => Self::Any(e),
        }
    }
}

/// Delete every row from the service's tables, for `db truncate` and the
/// request-suite setup.
///
/// `DELETE`, not `TRUNCATE`, and identities are not restarted.
/// Children-before-parent
/// order is kept by convention, though no table has a foreign key:
/// outbox, merge and audit rows and bulk jobs, then the organizations they
/// reference. The review queue is left alone, as it always was.
///
/// # Errors
///
/// When a delete fails. The deletes are separate statements, so the
/// tables before the failing one are already empty.
pub async fn delete_all_rows(db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM event_outbox").execute(db).await?;
    sqlx::query!("DELETE FROM merge_records")
        .execute(db)
        .await?;
    sqlx::query!("DELETE FROM audit_logs").execute(db).await?;
    sqlx::query!("DELETE FROM bulk_jobs").execute(db).await?;
    sqlx::query!("DELETE FROM organizations")
        .execute(db)
        .await?;
    Ok(())
}
