//! Shared application state injected into every REST handler.
//!
//! [`AppState`] bundles the database connection and the service singletons
//! (repository, event publisher, audit log, search engine, matcher, config)
//! behind `Arc`s so it is cheap to clone into Axum's per-request state. The
//! trait-object fields ([`WorkerRepository`], [`EventProducer`],
//! [`WorkerMatcher`]) keep handlers decoupled from concrete implementations.

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::config::Config;
use crate::db::{AuditLogRepository, SeaOrmWorkerRepository, WorkerRepository};
use crate::matching::{ProbabilisticMatcher, WorkerMatcher};
use crate::search::SearchEngine;
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
    /// Builds the application state from the long-lived dependencies the
    /// server owns. Internally it wires the secondary services together: an
    /// [`InMemoryEventPublisher`] is created first, then an
    /// [`AuditLogRepository`] over a clone of `db`, and finally a
    /// [`SeaOrmWorkerRepository`] configured with both so every write emits an
    /// event and an audit entry. All services are wrapped in `Arc`s (and the
    /// repository/matcher boxed as trait objects) so the returned [`AppState`]
    /// is cheap to clone into Axum's per-request state.
    #[must_use]
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
                .with_audit_log(audit_log.clone()),
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

/// Bridge so the existing `State<AppState>` handlers run as native loco
/// controllers: extracts the cheaply-cloneable `AppState` from the
/// `AppContext` shared store (populated once at boot in `after_routes`).
impl axum::extract::FromRef<loco_rs::app::AppContext> for AppState {
    fn from_ref(ctx: &loco_rs::app::AppContext) -> Self {
        ctx.shared_store
            .get::<AppState>()
            .expect("AppState must be inserted into the shared store at boot")
    }
}
