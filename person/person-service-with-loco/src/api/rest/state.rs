//! [`AppState`](crate::api::rest::state::AppState): the shared services every REST handler is given.
//!
//! Axum clones `AppState` per request, so every field is cheap to clone
//! (an `Arc` or a pooled connection). It bundles the database, person
//! repository, event publisher, audit log, search engine, matcher, and
//! configuration behind trait objects so handlers stay decoupled from
//! concrete implementations.

use authentication_verifier::Verifier;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::config::Config;
use crate::db::{AuditLogRepository, PersonRepository, SeaOrmPersonRepository};
use crate::matching::{PersonMatcher, ProbabilisticMatcher};
use crate::search::SearchEngine;
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

    /// Verifier for authentication-service RS256 bearer tokens. Defaults
    /// to an empty key set (rejects everything) so the service boots even
    /// when the JWKS source is unreachable; `after_routes` swaps in a
    /// populated verifier via [`AppState::with_verifier`] once the JWKS is
    /// fetched.
    pub verifier: Arc<Verifier>,
}

impl AppState {
    /// Assemble the shared state, wiring the repository to an in-memory
    /// event publisher and the audit log.
    ///
    /// Takes owned `search_engine`, `matcher`, and `config` and wraps
    /// them in `Arc`s. The repository is built with both the event
    /// publisher and audit log attached, so every mutation through it
    /// emits events and audit rows.
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

        // Create person repository with event publisher and audit log
        let person_repository = Arc::new(
            SeaOrmPersonRepository::new(db.clone())
                .with_event_publisher(event_publisher.clone())
                .with_audit_log(audit_log.clone()),
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
            verifier: Arc::new(empty_verifier()),
        }
    }

    /// Replace the token verifier (e.g. with one built from a freshly
    /// fetched JWKS at boot). Consumes and returns `self` for chaining.
    #[must_use]
    pub fn with_verifier(mut self, verifier: Arc<Verifier>) -> Self {
        self.verifier = verifier;
        self
    }
}

/// A verifier with no keys: every token is rejected until a real JWKS is
/// loaded. Infallible — an empty `keys` array always parses.
fn empty_verifier() -> Verifier {
    let empty = serde_json::json!({ "keys": [] });
    Verifier::from_jwks_value(&empty, "authentication-service", "main-x-service")
        .expect("empty jwks always builds")
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
