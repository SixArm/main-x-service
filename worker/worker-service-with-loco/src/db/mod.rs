//! Database layer: connection management, SeaORM entities, and repositories.
//!
//! Submodules: [`schema`](crate::db::schema) (table/column definitions),
//! [`models`](crate::db::models) (SeaORM entity structs),
//! [`repositories`](crate::db::repositories) (the
//! [`WorkerRepository`](crate::db::repositories::WorkerRepository) trait and
//! its SeaORM implementation), and [`audit`](crate::db::audit) (the
//! HIPAA-style audit-log repository). [`create_connection`](crate::db::create_connection)
//! builds the pooled [`DatabaseConnection`](sea_orm::DatabaseConnection) held
//! in `AppState`.
//!
//! Per the layering rules (spec §8.2), this `db` layer never depends on the
//! `api` layer; the API depends on it. The pooled connection is cheap to
//! `Clone` (it is internally reference-counted) and is shared across async
//! tasks; repositories borrow it rather than owning a fresh handle.

use sea_orm::{Database, DatabaseConnection};

use crate::Result;
use crate::config::DatabaseConfig;

/// HIPAA-style audit-log repository ([`audit::AuditLogRepository`]).
pub mod audit;
/// `chrono` <-> `time` conversions at the persistence boundary.
pub mod convert;
/// SeaORM entity modules (one per PostgreSQL table).
pub mod models;
/// [`repositories::WorkerRepository`] trait and its SeaORM implementation.
pub mod repositories;
/// Schema-definition stub retained for entity regeneration (see the module doc).
pub mod schema;

// Re-export the headline types so callers can `use crate::db::{...}` without
// reaching into the submodule paths.
pub use audit::AuditLogRepository;
pub use repositories::{AuditContext, SeaOrmWorkerRepository, WorkerRepository};

/// Opens a pooled `PostgreSQL` connection using the URL and pool bounds from
/// `config`. Maps connection failures to [`crate::Error::Pool`].
///
/// # Errors
///
/// Returns [`crate::Error::Pool`] (carrying the underlying `SeaORM` error text)
/// if the connection cannot be established — e.g. an unreachable host, bad
/// credentials, or a malformed connection URL.
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    // Configure the SeaORM connection pool from the database config: the URL
    // and the upper/lower bounds on pooled connections.
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);

    // Establish the pool eagerly; flatten SeaORM's `DbErr` into our `Error`.
    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}
