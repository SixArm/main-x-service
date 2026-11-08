//! Persistence: the row types and **every SQL statement this service
//! runs** (spec/tech-stack §5.2).
//!
//! Each module owns one table: a plain row struct and the helpers the
//! controllers, the tasks, the rate limiter and the address-retention
//! sweep call. Every statement is a `sqlx::query!` / `query_as!` /
//! `query_scalar!` literal, checked at compile time against the committed
//! `.sqlx/` data, so a reviewer reads the whole of the service's SQL in
//! this directory. The schema itself is the plain SQL under
//! `migrations/`, run by [`MIGRATOR`].
//!
//! A helper takes an [`axa::sqlx::Db`] (or a `&mut PgConnection` when it
//! only ever runs inside a caller's transaction), so the caller passes
//! either the pool or a connection it already holds.

use std::fmt;

/// Auth-event model — durable audit trail of authentication events.
pub mod auth_events;
/// The rate limiter's window log (`auth_rate_limits`).
pub mod auth_rate_limits;
/// Session model — cookie-session issuance, sliding TTLs and revocation.
pub mod sessions;
/// User model — passwordless magic-link accounts.
pub mod users;

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `authentication-service-cli db migrate | down | reset | status`
/// and the boot auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A model-layer failure.
///
/// The variants are the ones the `SeaORM` era's `axa::model::ModelError`
/// (and `DbErr`) were used for here, with the same names, so a caller's
/// match and a snapshot of the `Debug` text are unchanged. Each converts
/// into the [`axa::Error`] the old one became, so every HTTP answer is
/// unchanged too: a missing row is `404`, an existing one `409`, a
/// database error is axa's own mapping, and anything else is `500`.
#[derive(Debug)]
pub enum ModelError {
    /// The row already exists. `409`.
    EntityAlreadyExists,
    /// The row does not exist. `404`.
    EntityNotFound,
    /// A `users` row failed validation. Carries the JSON map of field name
    /// to error list that `SeaORM`'s `DbErr::Custom` carried, and is a
    /// `500` as that was.
    Custom(String),
    /// An update by primary key matched no row, as `SeaORM`'s
    /// `DbErr::RecordNotUpdated` reported. `500`.
    RecordNotUpdated,
    /// A free-form failure. `500`.
    Message(String),
    /// A database failure.
    Sqlx(sqlx::Error),
    /// Any other failure.
    Any(Box<dyn std::error::Error + Send + Sync>),
}

/// A model-layer result.
pub type ModelResult<T> = std::result::Result<T, ModelError>;

impl ModelError {
    /// A free-form failure.
    #[must_use]
    pub fn msg(message: &str) -> Self {
        Self::Message(message.to_string())
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EntityAlreadyExists => f.write_str("Entity already exists"),
            Self::EntityNotFound => f.write_str("Entity not found"),
            Self::Custom(text) => write!(f, "Custom Error: {text}"),
            Self::RecordNotUpdated => f.write_str("None of the records are updated"),
            Self::Message(text) => f.write_str(text),
            Self::Sqlx(e) => e.fmt(f),
            Self::Any(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ModelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Sqlx(e) => Some(e),
            Self::Any(e) => Some(e.as_ref()),
            _ => None,
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
            ModelError::EntityAlreadyExists => Self::CustomError(
                axum::http::StatusCode::CONFLICT,
                axa::prelude::ErrorDetail::new("conflict", "Resource already exists"),
            ),
            ModelError::Custom(_) | ModelError::RecordNotUpdated | ModelError::Message(_) => {
                Self::Message(e.to_string())
            }
            ModelError::Sqlx(e) => Self::Sqlx(e),
            ModelError::Any(e) => Self::Any(e),
        }
    }
}

/// A `LIMIT` bound: Postgres takes `BIGINT`, the callers count in `u64`.
pub(crate) fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Delete every row from the service's tables, for `db truncate` and the
/// test harness.
///
/// `DELETE`, not `TRUNCATE`, and identities are not restarted: exactly
/// what the `SeaORM` era's `truncate_table` did, on the same three tables
/// in the same order. `auth_rate_limits` was never truncated and still is
/// not; [`auth_rate_limits::delete_all`] clears it for the tests that need
/// it empty.
///
/// # Errors
///
/// When a delete fails. The deletes are separate statements, so the
/// tables before the failing one are already empty.
pub async fn delete_all_rows(db: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM auth_events").execute(db).await?;
    sqlx::query!("DELETE FROM sessions").execute(db).await?;
    sqlx::query!("DELETE FROM users").execute(db).await?;
    Ok(())
}
