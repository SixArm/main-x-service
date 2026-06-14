//! Database layer: connection management, SeaORM entities, repositories.
//!
//! [`create_connection`](crate::db::create_connection) builds a pooled
//! SeaORM [`DatabaseConnection`](sea_orm::DatabaseConnection) from
//! [`DatabaseConfig`](crate::config::DatabaseConfig). The submodules
//! cover the SeaORM entities ([`models`](crate::db::models)), the schema
//! definitions ([`schema`](crate::db::schema)), the repository trait +
//! impl ([`repositories`](crate::db::repositories)), and the audit-log
//! repository ([`audit`](crate::db::audit)).

use sea_orm::{Database, DatabaseConnection};

use crate::Result;
use crate::config::DatabaseConfig;

/// Audit-log repository.
pub mod audit;
/// chrono <-> time conversions at the persistence boundary.
pub mod convert;
/// SeaORM entity models.
pub mod models;
/// Repository trait and SeaORM implementation.
pub mod repositories;
/// Schema/table definitions.
pub mod schema;

pub use audit::AuditLogRepository;
pub use repositories::{AuditContext, EventRepository, SeaOrmEventRepository};

/// Open a pooled `SeaORM` connection using the configured URL and pool
/// bounds.
///
/// Reads `url`, `max_connections`, and `min_connections` from the
/// supplied [`DatabaseConfig`](crate::config::DatabaseConfig) and hands
/// them to `SeaORM`'s [`ConnectOptions`](sea_orm::ConnectOptions). The
/// returned [`DatabaseConnection`](sea_orm::DatabaseConnection) is
/// cheaply clonable and shared across the service via `Arc`-wrapped
/// repositories.
///
/// # Errors
///
/// Returns [`Error::Pool`](crate::Error::Pool) if the underlying
/// `Database::connect` fails (bad URL, unreachable server, auth
/// failure, exhausted pool, …); the driver error is stringified into
/// the variant.
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    // Seed connect options from the configured URL, then layer on the
    // pool bounds before opening the connection.
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);

    // Map any driver/pool error into the crate's typed `Error::Pool`.
    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}
