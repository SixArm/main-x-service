//! [`AppState`](crate::api::rest::state::AppState): the shared services every REST handler is given.
//!
//! Axum clones `AppState` per request, so every field is cheap to clone
//! (an `Arc` or a pooled connection). It bundles the database, person
//! repository, event publisher, audit log, search engine, matcher, and
//! configuration behind trait objects so handlers stay decoupled from
//! concrete implementations.

use std::sync::Arc;
use sea_orm::DatabaseConnection;

use crate::search::SearchEngine;
use crate::matching::{ProbabilisticMatcher, PersonMatcher};
use crate::config::Config;
use crate::db::{PersonRepository, SeaOrmPersonRepository, AuditLogRepository};
use crate::streaming::{EventProducer, InMemoryEventPublisher};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// Database connection
    pub db: DatabaseConnection,

    /// Person repository for database operations
    pub person_repository: Arc<dyn PersonRepository>,

    /// Event publisher for person events
    pub event_publisher: Arc<dyn EventProducer>,

    /// Audit log repository
    pub audit_log: Arc<AuditLogRepository>,

    /// Search engine for person lookups
    pub search_engine: Arc<SearchEngine>,

    /// Person matcher for finding duplicates
    pub matcher: Arc<dyn PersonMatcher>,

    /// Application configuration
    pub config: Arc<Config>,
}

impl AppState {
    /// Assemble the shared state, wiring the repository to an in-memory
    /// event publisher and the audit log.
    ///
    /// Takes owned `search_engine`, `matcher`, and `config` and wraps
    /// them in `Arc`s. The repository is built with both the event
    /// publisher and audit log attached, so every mutation through it
    /// emits events and audit rows.
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

        // Create person repository with event publisher and audit log
        let person_repository = Arc::new(
            SeaOrmPersonRepository::new(db.clone())
                .with_event_publisher(event_publisher.clone())
                .with_audit_log(audit_log.clone())
        ) as Arc<dyn PersonRepository>;

        let person_matcher = Arc::new(matcher) as Arc<dyn PersonMatcher>;

        Self {
            db,
            person_repository,
            event_publisher,
            audit_log,
            search_engine: Arc::new(search_engine),
            matcher: person_matcher,
            config: Arc::new(config),
        }
    }
}
