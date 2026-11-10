//! Persistence: the row types and **every SQL statement this service
//! runs** (spec/tech-stack §5.2).
//!
//! Patient Flow owns a **normalized relational schema** (PF-D3) — not
//! the matcher-DTO-as-JSONB shape of the entity registries — because
//! its value is constraints and transitions. Each module owns one
//! table: a plain row struct (`Model`, serialized as-is by the read
//! endpoints) and the statements the controllers, the seed task and the
//! truncate run against it. [`records`] keeps the shared active-row
//! finders; [`audit_logs`] and [`event_outbox`] are the audit /
//! durable-event side tables.
//!
//! Every statement is a `sqlx::query!` / `query_as!` / `query_scalar!`
//! literal, checked at compile time against the committed `.sqlx/`
//! data, so a reviewer reads the whole of the service's SQL in this
//! directory. The schema itself is the plain SQL under `migrations/`,
//! run by [`MIGRATOR`].
//!
//! A helper takes an [`axa::sqlx::Db`], so the caller passes either the
//! pool or a connection it already holds (`&mut *txn`). The handlers
//! pass their open transaction, so the row change, its audit entry and
//! (under the `outbox` transport) its event share one commit boundary
//! (PF-D9).

use std::fmt;

pub mod audit_logs;
pub mod bays;
pub mod bed_requests;
pub mod beds;
pub mod event_outbox;
pub mod infection_flags;
pub mod records;
pub mod red_green_days;
pub mod sites;
pub mod stays;
pub mod transfers;
pub mod wards;

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `patient-flow-service db migrate | down | reset | status` and
/// the boot auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A model-layer failure.
///
/// The variant names are stable, so a caller's match does not move, and each
/// converts into an [`axa::Error`] with a fixed status: a missing row is
/// `404`, a database error is axa's own mapping (a broken constraint is
/// `409`, anything else `500`), and any other failure is `500`.
#[derive(Debug)]
pub enum ModelError {
    /// The row does not exist (or is soft-deleted).
    EntityNotFound,
    /// A database failure.
    Sqlx(sqlx::Error),
    /// Any other failure (a malformed pid, a payload that does not
    /// serialize).
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

/// An update that matched no row is an error, not a silent success:
/// `RowNotFound`, which axa answers with `500`.
pub(crate) fn updated_one(rows_affected: u64) -> ModelResult<()> {
    if rows_affected == 0 {
        Err(ModelError::Sqlx(sqlx::Error::RowNotFound))
    } else {
        Ok(())
    }
}

/// Delete every row from the service's tables, for `db truncate` and the
/// test harness.
///
/// `DELETE`, not `TRUNCATE`, on all eleven tables, children before
/// parents.
///
/// # Errors
///
/// When a delete fails. The deletes are separate statements, so the
/// tables before the failing one are already empty.
pub async fn delete_all_rows(db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM event_outbox").execute(db).await?;
    sqlx::query!("DELETE FROM audit_logs").execute(db).await?;
    sqlx::query!("DELETE FROM infection_flags")
        .execute(db)
        .await?;
    sqlx::query!("DELETE FROM red_green_days")
        .execute(db)
        .await?;
    sqlx::query!("DELETE FROM bed_requests").execute(db).await?;
    sqlx::query!("DELETE FROM transfers").execute(db).await?;
    sqlx::query!("DELETE FROM stays").execute(db).await?;
    sqlx::query!("DELETE FROM beds").execute(db).await?;
    sqlx::query!("DELETE FROM bays").execute(db).await?;
    sqlx::query!("DELETE FROM wards").execute(db).await?;
    sqlx::query!("DELETE FROM sites").execute(db).await?;
    Ok(())
}
