//! Shared application state injected into every REST handler.
//!
//! [`AppState`] bundles the database connection and the service singletons
//! (repository, event publisher, audit log, search engine, matcher, config)
//! behind `Arc`s so it is cheap to clone into Axum's per-request state. The
//! trait-object fields ([`WorkerRepository`], [`EventProducer`],
//! [`WorkerMatcher`]) keep handlers decoupled from concrete implementations.

use authentication_verifier::Verifier;
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
                .with_audit_log(audit_log.clone())
                .with_transport(crate::streaming::transport()),
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

// The token verifier used to live here as an `Arc<Verifier>` snapshot,
// captured a second time by the enforcement layer. It moved to the
// process-wide reloadable holder in `super::auth::verifier()`, so a key
// rotation reaches the guard and the extractors together; two snapshots
// could only ever update one of them. The builders below still supply
// the key set that holder starts from.

/// Default issuer expected in tokens (`iss`).
pub(crate) const DEFAULT_ISSUER: &str = "authentication-service";
/// Default audience expected in tokens (`aud`).
pub(crate) const DEFAULT_AUDIENCE: &str = "main-x-service";

/// Read env var `name`, treating unset/blank as absent and falling back
/// to `default`. Used for the issuer/audience so a blank value doesn't
/// override the sensible default.
pub(crate) fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Build the PASETO token verifier from the environment:
///
/// - `WORKER_PASETO_KEYS` — the Ed25519 key set (JSON, OKP/Ed25519 form)
///   the auth service publishes at `/.well-known/paseto-keys`. Absent /
///   blank / unparseable ⇒ an empty key set, so every token is rejected
///   but the service still boots without credentials configured.
/// - `WORKER_TOKEN_ISSUER` — expected `iss` (default
///   `authentication-service`).
/// - `WORKER_TOKEN_AUDIENCE` — expected `aud` (default
///   `main-x-service`).
///
/// For the boot-time HTTP fetch of the key set
/// (`WORKER_PASETO_KEYS_URL`), see [`verifier_from_env_or_fetch`].
pub(crate) fn verifier_from_env() -> Verifier {
    let issuer = env_or("WORKER_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("WORKER_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    env_keys_verifier(&issuer, &audience)
}

/// The `WORKER_PASETO_KEYS` env-key-set path with an explicit issuer /
/// audience: parse the key-set JSON from the variable, or fall back to
/// an empty (reject-all) set so the service always boots.
fn env_keys_verifier(issuer: &str, audience: &str) -> Verifier {
    let keys = std::env::var("WORKER_PASETO_KEYS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    Verifier::from_paseto_keys_value(&keys, issuer, audience)
        .unwrap_or_else(|_| empty_verifier(issuer, audience))
}

/// Build the boot-time PASETO verifier, preferring an HTTP fetch of the
/// key set (spec §13 T-1b fetch item). Reads:
///
/// - `WORKER_PASETO_KEYS_URL` — URL of the auth-service published key
///   set (`/.well-known/paseto-keys`). **Unset/blank** ⇒ exactly the
///   env-key-set path ([`verifier_from_env`]). **Set** ⇒ fetch once at
///   boot; on success the fetched key set **wins** over
///   `WORKER_PASETO_KEYS`; on failure (network/HTTP/parse) log a
///   warning and fall back to the env path — the service always boots,
///   auth-service downtime never prevents startup.
/// - `WORKER_TOKEN_ISSUER` / `WORKER_TOKEN_AUDIENCE` — expected `iss` /
///   `aud`, same defaults as [`verifier_from_env`].
///
/// The fetch happens once at boot; there is no refresh loop (periodic
/// refresh is a possible future item — spec §15).
pub async fn verifier_from_env_or_fetch() -> Verifier {
    let issuer = env_or("WORKER_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("WORKER_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    let url = std::env::var("WORKER_PASETO_KEYS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    verifier_from_url_or_env(url.as_deref(), &issuer, &audience).await
}

/// Core of [`verifier_from_env_or_fetch`], parameterised on the URL so
/// it is unit-testable without touching the process environment.
/// `None` ⇒ the `WORKER_PASETO_KEYS` env path; `Some(url)` ⇒ fetch via
/// [`Verifier::from_paseto_keys_url`] — the fetched set wins on
/// success, the env path is the fallback on any fetch error. Never
/// panics; the service always gets a verifier.
pub async fn verifier_from_url_or_env(url: Option<&str>, issuer: &str, audience: &str) -> Verifier {
    let Some(url) = url else {
        return env_keys_verifier(issuer, audience);
    };
    match Verifier::from_paseto_keys_url(url, issuer, audience).await {
        Ok(verifier) => {
            tracing::info!(
                url,
                key_count = verifier.key_count(),
                "PASETO key set fetched at boot; fetched set overrides WORKER_PASETO_KEYS"
            );
            verifier
        }
        Err(error) => {
            tracing::warn!(
                url,
                error = %error,
                "PASETO key set fetch failed; falling back to the WORKER_PASETO_KEYS env path"
            );
            env_keys_verifier(issuer, audience)
        }
    }
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
            env_or("WORKER_SERVICE_TEST_UNSET_VAR_XYZ", "fallback"),
            "fallback"
        );
    }
}
