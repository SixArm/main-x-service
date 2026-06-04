//! Database access — connection pool + repository trait.
//!
//! The repository implementation is a STUB. Full SeaORM entities and
//! query construction land alongside the first iterations on
//! `spec.md §13` task queue.

use sea_orm::{Database, DatabaseConnection};

use crate::Result;
use crate::config::DatabaseConfig;
use crate::models::Course;

/// Open a connection pool from a `DatabaseConfig`.
pub async fn create_connection(config: &DatabaseConfig) -> Result<DatabaseConnection> {
    let mut opt = sea_orm::ConnectOptions::new(&config.url);
    opt.max_connections(config.max_connections)
        .min_connections(config.min_connections);
    Database::connect(opt)
        .await
        .map_err(|e| crate::Error::Pool(e.to_string()))
}

/// Repository for Course CRUD. STUB — every method returns
/// `Error::Api("not implemented")` until the SeaORM entities land.
///
/// `dyn`-compatibility provided by `async_trait` (object-safety
/// blocks plain `async fn` in trait methods until we move to RPITIT).
#[async_trait::async_trait]
pub trait CourseRepository: Send + Sync {
    async fn create(&self, course: &Course) -> Result<Course>;
    async fn get_by_id(&self, id: &uuid::Uuid) -> Result<Option<Course>>;
    async fn update(&self, course: &Course) -> Result<Course>;
    async fn soft_delete(&self, id: &uuid::Uuid) -> Result<()>;
    async fn list(&self, limit: u64, offset: u64) -> Result<Vec<Course>>;
}

/// Placeholder implementation. Wraps a `DatabaseConnection` so that the
/// real impl can be slotted in without touching call sites.
pub struct SeaOrmCourseRepository {
    #[allow(dead_code)]
    db: DatabaseConnection,
}

impl SeaOrmCourseRepository {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl CourseRepository for SeaOrmCourseRepository {
    async fn create(&self, _course: &Course) -> Result<Course> {
        Err(crate::Error::Api("CourseRepository::create not implemented".into()))
    }
    async fn get_by_id(&self, _id: &uuid::Uuid) -> Result<Option<Course>> {
        Err(crate::Error::Api("CourseRepository::get_by_id not implemented".into()))
    }
    async fn update(&self, _course: &Course) -> Result<Course> {
        Err(crate::Error::Api("CourseRepository::update not implemented".into()))
    }
    async fn soft_delete(&self, _id: &uuid::Uuid) -> Result<()> {
        Err(crate::Error::Api("CourseRepository::soft_delete not implemented".into()))
    }
    async fn list(&self, _limit: u64, _offset: u64) -> Result<Vec<Course>> {
        Err(crate::Error::Api("CourseRepository::list not implemented".into()))
    }
}
