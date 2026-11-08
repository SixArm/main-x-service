//! Database layer: the connection pool, the migrator, the repository, the
//! audit log, the outbox, the entity links, the review queue, the
//! assessments and erasure. **Every SQL statement this service runs is in
//! this directory** (spec/tech-stack §5.2), written as a `sqlx::query!` /
//! `query_as!` / `query_scalar!` literal so the compiler checks it against
//! the schema.
//!
//! Submodules: [`models`](crate::db::models) (row structs),
//! [`repositories`](crate::db::repositories) (the
//! [`WorkerRepository`](crate::db::repositories::WorkerRepository) trait and
//! its sqlx implementation), and [`audit`](crate::db::audit) (the
//! HIPAA-style audit-log repository). [`create_connection`](crate::db::create_connection)
//! builds the pooled [`PgPool`] held in `AppState`.
//!
//! Per the layering rules (spec §8.2), this `db` layer never depends on the
//! `api` layer; the API depends on it. The pool is cheap to `Clone` (it is
//! internally reference-counted) and is shared across async tasks.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::Result;
use crate::config::DatabaseConfig;

/// Workforce-assessment persistence (`worker_assessments`).
pub mod assessments;
/// HIPAA-style audit-log repository ([`audit::AuditLogRepository`]).
pub mod audit;
/// `chrono` <-> `time` conversions at the persistence boundary.
pub mod convert;
/// Cross-service entity-link write-side persistence (`same_identity`).
pub mod entity_links;
/// The statements of a GDPR Art. 17 erasure.
pub mod erasure;
/// Row structs, one per table read whole.
pub mod models;
/// `event_outbox` write + relay surface for the durable event bus (Phase 2).
pub mod outbox;
/// [`repositories::WorkerRepository`] trait and its sqlx implementation.
pub mod repositories;
pub mod review_queue;

// Re-export the headline types so callers can `use crate::db::{...}` without
// reaching into the submodule paths.
pub use audit::AuditLogRepository;
pub use repositories::{AuditContext, PgWorkerRepository, WorkerRepository};

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `worker-service db migrate | down | reset | status` and the boot
/// auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Opens a pooled `PostgreSQL` connection using the URL and pool bounds from
/// `config`. Maps connection failures to [`crate::Error::Pool`].
///
/// # Errors
///
/// Returns [`crate::Error::Pool`] (carrying the underlying driver error text)
/// if the connection cannot be established — e.g. an unreachable host, bad
/// credentials, or a malformed connection URL.
pub async fn create_connection(config: &DatabaseConfig) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect(&config.url)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}

/// "No row matched" on an update or soft delete, worded as `SeaORM`'s
/// `RecordNotUpdated` was, so the error a caller sees is unchanged.
pub(crate) fn not_updated() -> crate::Error {
    crate::Error::Database("None of the records are updated".to_string())
}

/// A `u64` count or limit as the `BIGINT` Postgres takes, saturating.
pub(crate) fn to_i64(n: u64) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}
