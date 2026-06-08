//! PostgreSQL persistence layer: connection setup, SeaORM entities, and
//! repositories.
//!
//! This module owns the database boundary. [`create_connection`](crate::db::create_connection) builds
//! a pooled [`DatabaseConnection`](sea_orm::DatabaseConnection) from [`DatabaseConfig`](crate::config::DatabaseConfig). Submodules:
//! [`models`](crate::db::models) holds the SeaORM entity definitions, [`schema`](crate::db::schema) the table
//! definitions, [`repositories`](crate::db::repositories) the data-access traits and their SeaORM
//! implementations ([`PersonRepository`](crate::db::repositories::PersonRepository) / [`SeaOrmPersonRepository`](crate::db::repositories::SeaOrmPersonRepository)),
//! and [`audit`](crate::db::audit) the HIPAA-style [`AuditLogRepository`](crate::db::audit::AuditLogRepository). The key traits
//! and types are re-exported here for convenience.

use sea_orm::{Database, DatabaseConnection};

use crate::config::DatabaseConfig;
use crate::Result;

/// Table/column schema definitions.
pub mod schema;
/// SeaORM entity (ActiveModel/Model) definitions.
pub mod models;
/// Repository traits and their SeaORM implementations.
pub mod repositories;
/// jiff <-> time conversions at the persistence boundary.
pub mod convert;
/// Audit-log repository for the HIPAA-style trail.
pub mod audit;

pub use repositories::{PersonRepository, SeaOrmPersonRepository, AuditContext};
pub use audit::AuditLogRepository;

/// Open a pooled PostgreSQL connection from the given configuration.
///
/// Applies the configured min/max pool sizes. Connection errors are
/// mapped to [`crate::Error::Pool`]. The returned
/// [`DatabaseConnection`] is cheap to clone and is shared across
/// handlers via [`AppState`](crate::api::rest::state::AppState).
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);

    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}
