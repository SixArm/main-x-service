//! Database access — the connection pool, the migrator, the repository,
//! the audit log, and the outbox. **Every SQL statement this service runs
//! is in this directory** (spec/tech-stack §5.2), written as a
//! `sqlx::query!` / `query_as!` / `query_scalar!` literal so the compiler
//! checks it against the schema.
//!
//! [`create_connection`] builds a pooled
//! [`PgPool`] from [`DatabaseConfig`]. The
//! submodules cover the row structs ([`models`]), the
//! repository trait + impl ([`repositories`]),
//! the audit-log repository ([`audit`]), and the
//! transactional outbox ([`outbox`]).

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::Result;
use crate::config::DatabaseConfig;

pub mod audit;
pub mod convert;
pub mod models;
pub mod outbox;
pub mod repositories;

pub use audit::AuditLogRepository;
pub use repositories::{AuditContext, EventRepository, PgEventRepository, rows_for_verification};

/// The schema migrations: the plain SQL files under `migrations/`, one
/// `<version>_<name>.up.sql` / `.down.sql` pair each, embedded at build
/// time. `event-service db migrate | down | reset | status` and the boot
/// auto-migrate run these through `axa::sqlx`.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Open a connection pool using the configured URL and pool bounds.
///
/// Reads `url`, `max_connections`, and `min_connections` from the
/// supplied [`DatabaseConfig`]. The
/// returned [`PgPool`] is cheaply clonable and shared across the service
/// via `Arc`-wrapped repositories.
///
/// # Errors
///
/// Returns [`Error::Pool`](crate::Error::Pool) if the connection fails
/// (bad URL, unreachable server, auth failure, …); the driver error is
/// stringified into the variant.
pub async fn create_connection(config: &DatabaseConfig) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect(&config.url)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}
