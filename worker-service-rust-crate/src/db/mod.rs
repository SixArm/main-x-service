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

use sea_orm::{Database, DatabaseConnection};

use crate::config::DatabaseConfig;
use crate::Result;

pub mod schema;
pub mod models;
pub mod repositories;
pub mod audit;

pub use repositories::{WorkerRepository, SeaOrmWorkerRepository, AuditContext};
pub use audit::AuditLogRepository;

/// Opens a pooled PostgreSQL connection using the URL and pool bounds from
/// `config`. Maps connection failures to [`crate::Error::Pool`].
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    // Configure the SeaORM connection pool from the database config.
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);

    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}
