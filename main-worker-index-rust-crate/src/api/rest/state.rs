//! Application state for REST API

use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::search::SearchEngine;
use crate::matching::{ProbabilisticMatcher, WorkerMatcher};
use crate::config::Config;
use crate::db::{WorkerRepository, SeaOrmWorkerRepository, AuditLogRepository};
use crate::streaming::{EventProducer, InMemoryEventPublisher};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection
    pub db: DatabaseConnection,

    /// Worker repository for database operations
    pub worker_repository: Arc<dyn WorkerRepository>,

    /// Event publisher for worker events
    pub event_publisher: Arc<dyn EventProducer>,

    /// Audit log repository
    pub audit_log: Arc<AuditLogRepository>,

    /// Search engine for worker lookups
    pub search_engine: Arc<SearchEngine>,

    /// Worker matcher for finding duplicates
    pub matcher: Arc<dyn WorkerMatcher>,

    /// Application configuration
    pub config: Arc<Config>,
}

impl AppState {
    /// Create a new application state
    pub fn new(
        db: DatabaseConnection,
        search_engine: SearchEngine,
        matcher: ProbabilisticMatcher,
        config: Config,
    ) -> Self {
        // Create event publisher
        let event_publisher = Arc::new(InMemoryEventPublisher::new()) as Arc<dyn EventProducer>;

        // Create audit log repository
        let audit_log = Arc::new(AuditLogRepository::new(db.clone()));

        // Create worker repository with event publisher and audit log
        let worker_repository = Arc::new(
            SeaOrmWorkerRepository::new(db.clone())
                .with_event_publisher(event_publisher.clone())
                .with_audit_log(audit_log.clone())
        ) as Arc<dyn WorkerRepository>;

        let worker_matcher = Arc::new(matcher) as Arc<dyn WorkerMatcher>;

        Self {
            db,
            worker_repository,
            event_publisher,
            audit_log,
            search_engine: Arc::new(search_engine),
            matcher: worker_matcher,
            config: Arc::new(config),
        }
    }
}
