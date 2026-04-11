//! Application state for REST API

use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::search::SearchEngine;
use crate::matching::{ProbabilisticMatcher, EventMatcher};
use crate::config::Config;
use crate::db::{EventRepository, SeaOrmEventRepository, AuditLogRepository};
use crate::streaming::{EventProducer, InMemoryEventPublisher};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection
    pub db: DatabaseConnection,

    /// Event repository for database operations
    pub event_repository: Arc<dyn EventRepository>,

    /// Event publisher for event events
    pub event_publisher: Arc<dyn EventProducer>,

    /// Audit log repository
    pub audit_log: Arc<AuditLogRepository>,

    /// Search engine for event lookups
    pub search_engine: Arc<SearchEngine>,

    /// Event matcher for finding duplicates
    pub matcher: Arc<dyn EventMatcher>,

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

        // Create event repository with event publisher and audit log
        let event_repository = Arc::new(
            SeaOrmEventRepository::new(db.clone())
                .with_event_publisher(event_publisher.clone())
                .with_audit_log(audit_log.clone())
        ) as Arc<dyn EventRepository>;

        let event_matcher = Arc::new(matcher) as Arc<dyn EventMatcher>;

        Self {
            db,
            event_repository,
            event_publisher,
            audit_log,
            search_engine: Arc::new(search_engine),
            matcher: event_matcher,
            config: Arc::new(config),
        }
    }
}
