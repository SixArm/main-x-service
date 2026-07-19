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
/// SeaORM entity (ActiveModel/Model) definitions.
pub mod models;
/// Transactional-outbox write + relay surface (durable event bus, Phase 2).
pub mod outbox;
pub mod review_queue;
/// Repository traits and their SeaORM implementations.
pub mod repositories;
/// Table/column schema definitions.
pub mod schema;

pub use audit::AuditLogRepository;
pub use repositories::{AuditContext, PersonRepository, SeaOrmPersonRepository};

/// Open a pooled `PostgreSQL` connection from the given configuration.
///
/// Applies the configured min/max pool sizes. Connection errors are
/// mapped to [`crate::Error::Pool`]. The returned
/// [`DatabaseConnection`] is cheap to clone and is shared across
/// handlers via [`AppState`](crate::api::rest::state::AppState).
///
/// # Errors
///
/// Returns [`crate::Error::Pool`] if the database connection cannot be
/// established.
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);

    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}
