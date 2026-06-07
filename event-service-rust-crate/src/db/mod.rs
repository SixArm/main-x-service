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

use crate::config::DatabaseConfig;
use crate::Result;

/// Schema/table definitions.
pub mod schema;
/// SeaORM entity models.
pub mod models;
/// Repository trait and SeaORM implementation.
pub mod repositories;
/// Audit-log repository.
pub mod audit;

pub use repositories::{EventRepository, SeaOrmEventRepository, AuditContext};
pub use audit::AuditLogRepository;

/// Open a pooled SeaORM connection using the configured URL and pool
/// bounds; pool failures surface as [`Error::Pool`](crate::Error::Pool).
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);

    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}
