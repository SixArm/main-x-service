//! Database access — the connection pool, the migrator, the repository,
//! the audit log, the outbox, the entity links, the bulk jobs, the review
//! queue and erasure. **Every SQL statement this service runs is in this
//! directory** (spec/tech-stack §5.2), written as a `sqlx::query!` /
//! `query_as!` / `query_scalar!` literal so the compiler checks it against
//! the schema.
//!
//! [`create_connection`] builds a pooled [`PgPool`] from
//! [`DatabaseConfig`]. The submodules cover the row structs ([`models`]),
//! the repository trait and its implementation ([`repositories`]:
//! [`PersonRepository`] / [`PgPersonRepository`]), and the HIPAA-style
//! [`AuditLogRepository`] ([`audit`]). The key traits and types are
//! re-exported here for convenience.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::Result;
use crate::config::DatabaseConfig;

/// Audit-log repository for the HIPAA-style trail.
pub mod audit;
/// `bulk_jobs` persistence helpers (bulk import/export).
pub mod bulk_jobs;
/// chrono <-> time conversions at the persistence boundary.
pub mod convert;
/// Cross-service entity-link write-side persistence (`same_identity`).
pub mod entity_links;
/// The statements of a GDPR Art. 17 erasure.
pub mod erasure;
/// Row structs, one per table read whole.
pub mod models;
/// Transactional-outbox write + relay surface (durable event bus, Phase 2).
pub mod outbox;
/// Repository traits and their sqlx implementations.
pub mod repositories;
pub mod review_queue;

pub use audit::AuditLogRepository;
pub use repositories::{AuditContext, PersonRepository, PgPersonRepository};

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `person-service db migrate | down | reset | status` and the boot
/// auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open a pooled `PostgreSQL` connection from the given configuration.
///
/// Applies the configured min/max pool sizes. Connection errors are
/// mapped to [`crate::Error::Pool`]. The returned [`PgPool`] is cheap to
/// clone and is shared across handlers via
/// [`AppState`](crate::api::rest::state::AppState).
///
/// # Errors
///
/// Returns [`crate::Error::Pool`] if the database connection cannot be
/// established.
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
