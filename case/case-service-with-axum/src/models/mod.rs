//! Persistence: the table row types and **every SQL statement this
//! service runs** (spec/tech-stack §5.2).
//!
//! Each module owns one table: a plain row struct, and the helpers the
//! controllers, the compliance controls, the relay, the bulk worker and
//! the tasks call. Every statement is a `sqlx::query!` / `query_as!` /
//! `query_scalar!` literal, checked at compile time against the committed
//! `.sqlx/` data, so a reviewer reads the whole of the service's SQL in
//! this directory. The schema itself is the plain SQL under
//! `migrations/`, run by [`MIGRATOR`].
//!
//! A helper that writes takes an [`axa::sqlx::Db`], so the caller passes
//! either the pool or its own open transaction (`&mut *txn`). That is how
//! a case row, its `event_outbox` row and its chained audit row commit in
//! one transaction under the `outbox` transport.

use std::fmt;

/// Audit-trail recording, the tamper-evident hash chain, and the queries
/// over `audit_logs`.
pub mod audit_logs;
/// Durable state for one asynchronous bulk import/export operation
/// (BLK-5; `agents/share/bulk-import-export.md` §3), plus the lost-job
/// reconciliation statement.
pub mod bulk_jobs;
/// CRUD/search helpers over `cases` (stored `case_matcher::Case`
/// payload), including the record-integrity and erasure writes.
pub mod cases;
/// The cross-service write side over `entity_links`.
pub mod entity_links;
/// Durable event-bus transactional-outbox write + relay helpers over
/// `event_outbox`.
pub mod event_outbox;
/// Merge-history recording and query helpers over `merge_records`.
pub mod merge_records;
/// The stored duplicate-review queue, including keyless bulk-import
/// duplicates (BLK-5; `agents/share/bulk-import-export.md` §6).
pub mod review_queue;

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `case-service db migrate | down | reset | status` and the boot
/// auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A model-layer failure.
///
/// The variants are the three the `SeaORM` era's `axa::model::ModelError`
/// was used for here, and each converts into the [`axa::Error`] the old
/// one became, so every HTTP answer is unchanged: a missing row is `404`,
/// a database error is axa's own mapping (a violated unique constraint is
/// `409`, anything else `500`), and any other failure is `500`.
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

/// A partial update that looked its row up first hit no row: the row is
/// gone, which the `SeaORM` era reported as [`ModelError::EntityNotFound`]
/// from that lookup (`404`).
pub(crate) fn found(rows_affected: u64) -> ModelResult<()> {
    if rows_affected == 0 {
        Err(ModelError::EntityNotFound)
    } else {
        Ok(())
    }
}

/// A whole-row update of a row already in hand hit no row. `SeaORM`
/// answered `RecordNotUpdated` there, a database error (`500`); this
/// keeps it a database error (`RowNotFound`, also `500`) rather than
/// turning it into a `404`.
pub(crate) fn updated(rows_affected: u64) -> ModelResult<()> {
    if rows_affected == 0 {
        Err(ModelError::Sqlx(sqlx::Error::RowNotFound))
    } else {
        Ok(())
    }
}

/// A `LIMIT` / `OFFSET` bound: Postgres takes `BIGINT`, the callers count
/// in `u64`.
pub(crate) fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Delete every row from the service's link, merge, audit and case
/// tables, for `db truncate` and the request-suite setup.
///
/// `DELETE`, not `TRUNCATE`, and identities are not restarted: exactly
/// what the `SeaORM` era's `truncate_table` did, on exactly the tables it
/// named, in the same order. `event_outbox`, `bulk_jobs` and
/// `review_queue` are left alone, as they always were.
///
/// # Errors
///
/// When a delete fails. The deletes are separate statements, so the
/// tables before the failing one are already empty.
pub async fn delete_all_rows(db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM entity_links").execute(db).await?;
    sqlx::query!("DELETE FROM merge_records")
        .execute(db)
        .await?;
    sqlx::query!("DELETE FROM audit_logs").execute(db).await?;
    sqlx::query!("DELETE FROM cases").execute(db).await?;
    Ok(())
}
