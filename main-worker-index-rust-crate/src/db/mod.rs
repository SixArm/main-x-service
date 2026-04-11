//! Database operations and connection management

use sea_orm::{Database, DatabaseConnection};

use crate::config::DatabaseConfig;
use crate::Result;

pub mod schema;
pub mod models;
pub mod repositories;
pub mod audit;

pub use repositories::{WorkerRepository, SeaOrmWorkerRepository, AuditContext};
pub use audit::AuditLogRepository;

/// Create a database connection
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);

    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}
