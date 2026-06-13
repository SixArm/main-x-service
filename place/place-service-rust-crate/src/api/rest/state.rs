//! Shared application state passed to every REST handler.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::db::{AuditLogRepository, PlaceRepository, SeaOrmPlaceRepository};
use crate::matching::PlaceMatcher;
use crate::search::SearchEngine;
use crate::streaming::{EventPublisher, InMemoryEventPublisher};

/// Shared, cheaply-cloneable handle to every service the REST handlers
/// need. Cloned per request by Axum; the inner services are `Arc`-shared.
#[derive(Clone)]
pub struct AppState {
    /// Raw connection pool (for ad-hoc queries outside the repository).
    pub db: DatabaseConnection,
    /// Place CRUD repository.
    pub place_repository: Arc<dyn PlaceRepository>,
    /// Audit-log repository.
    pub audit_log: Arc<AuditLogRepository>,
    /// Event-stream publisher.
    pub event_publisher: Arc<dyn EventPublisher>,
    /// Full-text search engine.
    pub search_engine: Arc<SearchEngine>,
    /// Record matcher.
    pub matcher: Arc<PlaceMatcher>,
    /// Loaded service configuration.
    pub config: Arc<Config>,
}

impl AppState {
    /// Assemble state from externally-built services, constructing the
    /// SeaORM repository, audit log, and in-memory event publisher from the
    /// shared connection.
    pub fn new(
        db: DatabaseConnection,
        search_engine: SearchEngine,
        matcher: PlaceMatcher,
        config: Config,
    ) -> Self {
        let place_repository: Arc<dyn PlaceRepository> =
            Arc::new(SeaOrmPlaceRepository::new(db.clone()));
        let audit_log = Arc::new(AuditLogRepository::new(db.clone()));
        let event_publisher: Arc<dyn EventPublisher> = Arc::new(InMemoryEventPublisher::new());
        Self {
            db,
            place_repository,
            audit_log,
            event_publisher,
            search_engine: Arc::new(search_engine),
            matcher: Arc::new(matcher),
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
