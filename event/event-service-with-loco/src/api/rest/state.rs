//! Shared application state for the REST API.
//!
//! [`AppState`](crate::api::rest::AppState) bundles every service an
//! Axum handler needs — the DB connection, the event repository, the
//! event publisher, the audit log, the search engine, the matcher, and
//! the config — behind cheap [`Arc`](std::sync::Arc) clones (the struct
//! derives [`Clone`]). One instance is built at startup and shared
//! across all requests via Axum's `with_state`.

use authentication_verifier::{Policy, Verifier};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::config::Config;
use crate::db::{AuditLogRepository, EventRepository, SeaOrmEventRepository};
use crate::matching::{EventMatcher, ProbabilisticMatcher};
use crate::search::SearchEngine;
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

    /// Verifier for authentication-service PASETO `v4.public` bearer
    /// tokens, checked offline against the published Ed25519 key set.
    /// Built from the environment at construction (see
    /// [`verifier_from_env`]); with no key set configured it holds an
    /// empty key set (rejects everything) so the service still boots.
    /// [`AppState::with_verifier`] can swap in a replacement (e.g. one
    /// built from a freshly fetched key set).
    pub verifier: Arc<Verifier>,

    /// Whether blanket `/api/*` bearer-token enforcement is on.
    /// Read once from `EVENT_REQUIRE_AUTH` at construction (see
    /// [`super::auth::require_auth_from_env`]) — **off by default**;
    /// restart the service to change it. When on, the
    /// [`super::auth::require_auth_mw`] middleware requires a valid
    /// PASETO bearer token on every route except the public allow-list
    /// (`super::auth::is_public_path`) — deny-unless-public.
    pub require_auth: bool,

    /// ABAC policy evaluated on verified tokens inside the blanket
    /// guard (so only when [`AppState::require_auth`] is on). Read
    /// once from `EVENT_ABAC_POLICY` / `EVENT_ABAC_POLICY_FILE` at
    /// construction (see [`super::auth::policy_from_env`]) — unset or
    /// unparsable ⇒ the built-in default policy; restart the service
    /// to change it.
    pub policy: Arc<Policy>,
}

impl AppState {
    /// Assemble an [`AppState`] from the four externally-built pieces
    /// (DB connection, search engine, matcher, config). Internally
    /// wires an in-memory event publisher and an audit-log repository,
    /// then constructs the [`SeaOrmEventRepository`] with both attached
    /// so every CRUD write also streams an event and records an audit
    /// entry.
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

        // Create event repository with event publisher and audit log
        let event_repository = Arc::new(
            SeaOrmEventRepository::new(db.clone())
                .with_event_publisher(event_publisher.clone())
                .with_audit_log(audit_log.clone()),
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
            verifier: Arc::new(verifier_from_env()),
            require_auth: super::auth::require_auth_from_env(),
            policy: Arc::new(super::auth::policy_from_env()),
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
/// - `EVENT_PASETO_KEYS` — the Ed25519 key set (JSON, OKP/Ed25519 form)
///   the auth service publishes at `/.well-known/paseto-keys`. Absent /
///   blank / unparseable ⇒ an empty key set, so every token is rejected
///   but the service still boots without credentials configured.
/// - `EVENT_TOKEN_ISSUER` — expected `iss` (default
///   `authentication-service`).
/// - `EVENT_TOKEN_AUDIENCE` — expected `aud` (default
///   `main-x-service`).
///
/// This is the env-injection path. Prefer [`boot_verifier`], which
/// fetches the key set over HTTP at boot when `EVENT_PASETO_KEYS_URL`
/// is set and falls back to this path otherwise.
fn verifier_from_env() -> Verifier {
    let issuer = env_or("EVENT_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("EVENT_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    let keys = std::env::var("EVENT_PASETO_KEYS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    Verifier::from_paseto_keys_value(&keys, &issuer, &audience)
        .unwrap_or_else(|_| empty_verifier(&issuer, &audience))
}

/// Build the PASETO token verifier at boot (async — call from an async
/// boot hook such as `App::after_routes`, **before** the routers /
/// enforcement middleware capture the verifier):
///
/// - `EVENT_PASETO_KEYS_URL` **set (non-blank)** — fetch the key-set
///   JSON over HTTP from the auth service (normally its
///   `/.well-known/paseto-keys` endpoint), once, at boot. A successful
///   fetch **wins** over any `EVENT_PASETO_KEYS` env value; a failed
///   fetch logs a warning and falls back to the env path. See
///   [`verifier_from_url_or_env`].
/// - `EVENT_PASETO_KEYS_URL` **unset / blank** — exactly the previous
///   behaviour: build from the `EVENT_PASETO_KEYS` env key set, else an
///   empty reject-all key set.
///
/// Either way the service always boots. The fetch happens **once**;
/// there is no refresh loop (re-fetch on key rotation is a roadmap
/// item — spec §15).
pub async fn boot_verifier() -> Verifier {
    match std::env::var("EVENT_PASETO_KEYS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        Some(url) => verifier_from_url_or_env(url.trim()).await,
        None => verifier_from_env(),
    }
}

/// Fetch the PASETO key set from `url` (via
/// `Verifier::from_paseto_keys_url`, the `authentication-verifier`
/// `fetch` feature) and build a verifier expecting the
/// `EVENT_TOKEN_ISSUER` / `EVENT_TOKEN_AUDIENCE` claims (same defaults
/// as [`verifier_from_env`]). On success the fetched key set wins (an
/// info log records the source); on any transport / status / parse
/// failure a warning is logged and the [`verifier_from_env`] path is
/// used instead — the service always boots.
pub async fn verifier_from_url_or_env(url: &str) -> Verifier {
    let issuer = env_or("EVENT_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("EVENT_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    match Verifier::from_paseto_keys_url(url, &issuer, &audience).await {
        Ok(verifier) => {
            tracing::info!(
                url,
                keys = verifier.key_count(),
                "PASETO key set fetched from the auth service at boot"
            );
            verifier
        }
        Err(error) => {
            tracing::warn!(
                url,
                error = %error,
                "boot-time PASETO key-set fetch failed; falling back to the EVENT_PASETO_KEYS env path"
            );
            verifier_from_env()
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
            env_or("EVENT_SERVICE_TEST_UNSET_VAR_XYZ", "fallback"),
            "fallback"
        );
    }
}
