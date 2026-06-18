//! Shared application state passed to every REST handler.

use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::config::Config;
use crate::db::{AuditLogRepository, SeaOrmThingRepository, ThingRepository};
use crate::matching::ThingMatcher;
use crate::search::SearchEngine;
use crate::streaming::{EventPublisher, InMemoryEventPublisher};

/// Shared, cheaply-cloneable handle to every service the REST handlers
/// need. Cloned per request by Axum; the inner services are `Arc`-shared.
#[derive(Clone)]
pub struct AppState {
    /// Raw connection pool (for ad-hoc queries outside the repository).
    pub db: DatabaseConnection,
    /// Thing CRUD repository.
    pub thing_repository: Arc<dyn ThingRepository>,
    /// Audit-log repository.
    pub audit_log: Arc<AuditLogRepository>,
    /// Event-stream publisher.
    pub event_publisher: Arc<dyn EventPublisher>,
    /// Full-text search engine.
    pub search_engine: Arc<SearchEngine>,
    /// Record matcher.
    pub matcher: Arc<ThingMatcher>,
    /// Loaded service configuration.
    pub config: Arc<Config>,
}

impl AppState {
    /// Assemble state from externally-built services, constructing the
    /// `SeaORM` repository, audit log, and in-memory event publisher from the
    /// shared connection.
    #[must_use]
    pub fn new(
        db: DatabaseConnection,
        search_engine: SearchEngine,
        matcher: ThingMatcher,
        config: Config,
    ) -> Self {
        let thing_repository: Arc<dyn ThingRepository> =
            Arc::new(SeaOrmThingRepository::new(db.clone()));
        let audit_log = Arc::new(AuditLogRepository::new(db.clone()));
        let event_publisher: Arc<dyn EventPublisher> = Arc::new(InMemoryEventPublisher::new());
        Self {
            db,
            thing_repository,
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
    /// Pull the boot-time `AppState` out of the shared store. The clone is
    /// cheap because every field is an `Arc`/handle.
    ///
    /// # Panics
    ///
    /// Panics if `after_routes` never inserted the state — a programmer
    /// error that would otherwise surface as a confusing per-request 500,
    /// so it fails loudly at the first request instead.
    fn from_ref(ctx: &loco_rs::app::AppContext) -> Self {
        ctx.shared_store
            .get::<AppState>()
            .expect("AppState must be inserted into the shared store at boot")
    }
}
