//! Shared application state passed to every REST handler.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::db::{CourseRepository, SeaOrmCourseRepository};
use crate::matching::CourseMatcher;
use crate::search::SearchEngine;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub course_repository: Arc<dyn CourseRepository>,
    pub search_engine: Arc<SearchEngine>,
    pub matcher: Arc<CourseMatcher>,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(
        db: DatabaseConnection,
        search_engine: SearchEngine,
        matcher: CourseMatcher,
        config: Config,
    ) -> Self {
        let course_repository: Arc<dyn CourseRepository> =
            Arc::new(SeaOrmCourseRepository::new(db.clone()));
        Self {
            db,
            course_repository,
            search_engine: Arc::new(search_engine),
            matcher: Arc::new(matcher),
            config: Arc::new(config),
        }
    }
}
