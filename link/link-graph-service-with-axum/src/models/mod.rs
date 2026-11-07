//! Persistence: the derived read-model's row types and **every SQL
//! statement this service runs** (spec/tech-stack §5.2; spec §5, §10).
//!
//! Each module owns one table: a plain row struct, and the helpers the
//! apply seam, the controllers, reconciliation, the probe and the
//! suggestion job call. Every statement is a `sqlx::query!` /
//! `query_as!` / `query_scalar!` literal, checked at compile time against
//! the committed `.sqlx/` data, so a reviewer reads the whole of the
//! service's SQL in this directory. The schema itself is the plain SQL
//! under `migrations/`, run by [`MIGRATOR`].
//!
//! A helper takes an [`axa::sqlx::Db`], so the caller passes either the
//! pool or a connection it already holds (`&mut *conn`, or an open
//! transaction). A helper that runs several statements takes one
//! connection for the length of the call and passes it on.

use std::fmt;

pub mod audit_log;
pub mod consumer_offsets;
pub mod edges;
pub mod entity_presence;
pub mod processed_events;
pub mod suggestion_runs;

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `link-graph-service db migrate | down | reset | status` and the
/// boot auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A model-layer failure.
///
/// The variants are the ones the `SeaORM` era's `axa::model::ModelError`
/// was used for here, and each converts into the [`axa::Error`] the old
/// one became, so every HTTP answer is unchanged: a missing row is `404`,
/// a database error is axa's own mapping, and any other failure (an
/// upstream fetch in reconciliation or the suggestion job) is `500`.
#[derive(Debug)]
pub enum ModelError {
    /// The row does not exist.
    EntityNotFound,
    /// A database failure.
    Sqlx(sqlx::Error),
    /// Any other failure.
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

/// A `LIMIT` bound: Postgres takes `BIGINT`, the callers count in `u64`.
pub(crate) fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Delete every row from the read-model tables, for `db truncate` and the
/// test harness.
///
/// `DELETE`, not `TRUNCATE`: exactly what the `SeaORM` era's
/// `truncate_table` did, on the same five tables in the same order.
/// `processed_events` was never in that list and is still left alone.
///
/// # Errors
///
/// When a delete fails. The deletes are separate statements, so the
/// tables before the failing one are already empty.
pub async fn delete_all_rows(db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM edges").execute(db).await?;
    sqlx::query!("DELETE FROM entity_presence")
        .execute(db)
        .await?;
    sqlx::query!("DELETE FROM consumer_offsets")
        .execute(db)
        .await?;
    sqlx::query!("DELETE FROM audit_log").execute(db).await?;
    sqlx::query!("DELETE FROM suggestion_runs")
        .execute(db)
        .await?;
    Ok(())
}
