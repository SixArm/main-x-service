//! Shared application state passed to every REST handler.

use std::sync::Arc;

use authentication_verifier::Verifier;
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
    /// Verifier for authentication-service PASETO `v4.public` bearer
    /// tokens, checked offline against the published Ed25519 key set.
    /// Built from the environment at construction (see
    /// [`verifier_from_env`]); with no key set configured it holds an
    /// empty key set (rejects everything) so the service still boots.
    /// [`AppState::with_verifier`] can swap in a replacement (e.g. one
    /// built from a freshly fetched key set).
    pub verifier: Arc<Verifier>,
}

impl AppState {
    /// Assemble state from externally-built services, constructing the
    /// `SeaORM` repository, audit log, and in-memory event publisher from the
    /// shared connection.
    #[must_use]
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
            verifier: Arc::new(verifier_from_env()),
        }
    }

    /// Replace the token verifier (e.g. with one built from a freshly
    /// fetched Ed25519 key set at boot). Consumes and returns `self` for
    /// chaining.
    #[must_use]
    pub fn with_verifier(mut self, verifier: Arc<Verifier>) -> Self {
        self.verifier = verifier;
        self
    }
}

/// Default issuer expected in tokens (`iss`).
const DEFAULT_ISSUER: &str = "authentication-service";
/// Default audience expected in tokens (`aud`).
const DEFAULT_AUDIENCE: &str = "main-x-service";

/// Read env var `name`, treating unset/blank as absent and falling back
/// to `default`. Used for the issuer/audience so a blank value doesn't
/// override the sensible default.
fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Build the PASETO token verifier from the environment:
///
/// - `PLACE_PASETO_KEYS` — the Ed25519 key set (JSON, OKP/Ed25519 form)
///   the auth service publishes at `/.well-known/paseto-keys`. Absent /
///   blank / unparseable ⇒ an empty key set, so every token is rejected
///   but the service still boots without credentials configured.
/// - `PLACE_TOKEN_ISSUER` — expected `iss` (default
///   `authentication-service`).
/// - `PLACE_TOKEN_AUDIENCE` — expected `aud` (default
///   `main-x-service`).
///
/// Fetching the key set over HTTP from the auth service at boot (instead
/// of injecting it via env) is a follow-up — see spec §13.
fn verifier_from_env() -> Verifier {
    let issuer = env_or("PLACE_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("PLACE_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    let keys = std::env::var("PLACE_PASETO_KEYS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    Verifier::from_paseto_keys_value(&keys, &issuer, &audience)
        .unwrap_or_else(|_| empty_verifier(&issuer, &audience))
}

/// A verifier with no keys: every token is rejected until a real key set
/// is configured. Infallible — an empty `keys` array always parses.
fn empty_verifier(issuer: &str, audience: &str) -> Verifier {
    let empty = serde_json::json!({ "keys": [] });
    Verifier::from_paseto_keys_value(&empty, issuer, audience).expect("empty key set always builds")
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The no-key fallback verifier always builds and holds zero keys,
    /// so every token is rejected until a real key set is configured.
    #[test]
    fn test_empty_verifier_builds_with_zero_keys() {
        let v = empty_verifier(DEFAULT_ISSUER, DEFAULT_AUDIENCE);
        assert_eq!(v.key_count(), 0);
        assert!(v.verify("v4.public.not-a-real-token").is_err());
    }

    /// `env_or` falls back to the default when the variable is unset.
    #[test]
    fn test_env_or_falls_back_when_unset() {
        assert_eq!(
            env_or("PLACE_SERVICE_TEST_UNSET_VAR_XYZ", "fallback"),
            "fallback"
        );
    }
}
