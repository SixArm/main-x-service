//! Application state for REST API

use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::search::SearchEngine;
use crate::matching::{ProbabilisticMatcher, PatientMatcher};
use crate::config::Config;
use crate::db::{PatientRepository, SeaOrmPatientRepository, AuditLogRepository};
use crate::streaming::{EventProducer, InMemoryEventPublisher};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection
    pub db: DatabaseConnection,

    /// Patient repository for database operations
    pub patient_repository: Arc<dyn PatientRepository>,

    /// Event publisher for patient events
    pub event_publisher: Arc<dyn EventProducer>,

    /// Audit log repository
    pub audit_log: Arc<AuditLogRepository>,

    /// Search engine for patient lookups
    pub search_engine: Arc<SearchEngine>,

    /// Patient matcher for finding duplicates
    pub matcher: Arc<dyn PatientMatcher>,

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

        // Create patient repository with event publisher and audit log
        let patient_repository = Arc::new(
            SeaOrmPatientRepository::new(db.clone())
                .with_event_publisher(event_publisher.clone())
                .with_audit_log(audit_log.clone())
        ) as Arc<dyn PatientRepository>;

        let patient_matcher = Arc::new(matcher) as Arc<dyn PatientMatcher>;

        Self {
            db,
            patient_repository,
            event_publisher,
            audit_log,
            search_engine: Arc::new(search_engine),
            matcher: patient_matcher,
            config: Arc::new(config),
        }
    }
}
