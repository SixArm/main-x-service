//! Bearer-token authentication for the portfolio API.
//!
//! [`AuthUser`] is an Axum extractor that pulls `Authorization: Bearer
//! <paseto>`, verifies the PASETO `v4.public` (Ed25519) signature, issuer,
//! audience and expiry against the
//! [authentication-service](../../../authentication/authentication-service-with-loco)
//! published key set, and yields the verified [`Claims`]. Verification is
//! stateless and offline — no database hit, no introspection call — so any
//! handler can require authentication by taking an `AuthUser` argument, and
//! a handler that wants the caller identity *when present* (e.g. to stamp
//! an audit `actor`) takes [`MaybeAuthUser`] instead.
//!
//! ## Key source
//!
//! The process-wide [`verifier`] is seeded once at boot ([`init`] is
//! called from `App::after_routes`, before the app serves traffic) and
//! built from the environment:
//!
//! - `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL` — optional URL of the auth service's
//!   published key set (`/.well-known/paseto-keys`). Set (non-blank) ⇒
//!   the key set is fetched over HTTP **once at boot** via
//!   [`Verifier::from_paseto_keys_url`]; on success the fetched key set
//!   wins over `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` (`tracing::info!`), on failure the
//!   service logs a `tracing::warn!` and falls back to the env path
//!   below — the service always boots. There is no refresh loop; a
//!   rotation-triggered refetch is a future spec item.
//! - `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` — the Ed25519 key set (JSON, OKP/Ed25519
//!   JWK form) the auth service publishes at `/.well-known/paseto-keys`.
//!   Absent ⇒ an empty key set, so every token is rejected (the service
//!   still boots).
//! - `PROJECT_PORTFOLIO_MANAGEMENT_TOKEN_ISSUER` — expected `iss` (default
//!   `authentication-service`).
//! - `PROJECT_PORTFOLIO_MANAGEMENT_TOKEN_AUDIENCE` — expected `aud` (default
//!   `main-x-service`).
//!
//! ## Blanket enforcement
//!
//! When `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
//! case-insensitive), the [`enforce`] decision — wired as an Axum
//! middleware layer in `src/app.rs` — requires a valid bearer token on
//! every route except the public health/ping, OpenAPI/Swagger, and
//! Prometheus metrics paths (see [`is_public_path`]). It is **off by
//! default**: unset/blank/junk
//! ⇒ today's behaviour, where the extractor is opt-in per handler and
//! `GET /api/projects/whoami` proves end-to-end verification.
//! Activation is an operations decision once the SSO token flow is live;
//! see `agents/share/authentication-sessions.md` and
//! `agents/share/jwt-enforcement.md` for the family-wide contract.
//!
//! ## Authorization (ABAC)
//!
//! Inside the same guard — so it applies only when
//! `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` is on — a verified token is further checked
//! against an **attribute-based access control** policy per
//! `agents/share/authorization-attributes.md`: the request's action is
//! derived from the HTTP method plus this crate's destructive named
//! POSTs ([`DESTRUCTIVE_POST_SUFFIXES`]), and the shared engine in the
//! `authentication-verifier` crate evaluates the policy over the
//! token's `attrs` claim. The four collections (portfolios / projects /
//! products / programs) share one guard and one [`ENTITY`] in v1. The
//! policy is read once at first use ([`policy`], built by
//! [`policy_from_env`]) from `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY` (inline JSON) or
//! `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE` (path); unset or unparsable ⇒ the
//! built-in default policy (`svc=true` ⇒ everything; `access=admin` ⇒
//! destructive+write; `access=write` ⇒ write; otherwise read-only) —
//! the service always boots. **401** = missing/bad credential; **403**
//! = valid credential, policy denied (body carries the deciding rule).

use std::sync::{Arc, OnceLock};

use authentication_verifier::{Action, Claims, Policy, Verifier};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};

/// The resource entity this crate guards, as seen by ABAC policies
/// (the `entity` pseudo-attribute in rule `when` clauses). The four
/// REST collections (portfolios / projects / products / programs) are
/// one entity to the guard in v1 — do not per-collection this.
pub const ENTITY: &str = "portfolio";

/// Path suffixes of this crate's **destructive named POSTs** (per
/// `authorization-attributes.md` §2), fixed family-wide: record
/// **merge** (`POST /api/{collection}/merge`, live today), batch
/// **deduplicate** and bulk **import** (`POST
/// /api/{collection}/import` per `bulk-import-export.md` §4 — both
/// deferred spec §13 items whose routes end with these suffixes when
/// they land). A POST whose path ends with one of these derives
/// [`Action::Destructive`] instead of [`Action::Write`]; the
/// import-job **status** reads (`GET …/import/{id}`) are GETs and
/// derive [`Action::Read`] — only POST consults this list.
pub const DESTRUCTIVE_POST_SUFFIXES: [&str; 3] = ["/merge", "/deduplicate", "/import"];

/// Default issuer expected in tokens (`iss`).
const DEFAULT_ISSUER: &str = "authentication-service";
/// Default audience expected in tokens (`aud`).
const DEFAULT_AUDIENCE: &str = "main-x-service";

/// The process-wide token verifier, seeded by [`init`] at boot (or, if
/// [`init`] never ran — e.g. in unit tests — built lazily from the
/// environment on first use). Shared behind an `Arc` and read-only.
static VERIFIER: OnceLock<Arc<Verifier>> = OnceLock::new();

/// The process-wide token verifier (see [`VERIFIER`] and the module
/// docs). Shared behind an `Arc` and read-only.
#[must_use]
pub fn verifier() -> &'static Arc<Verifier> {
    VERIFIER.get_or_init(|| Arc::new(build_from_env()))
}

/// Seed the process-wide [`verifier`] before the app serves traffic
/// (called from `App::after_routes`). When `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL` is
/// set (non-blank) the published key set is fetched over HTTP **once** —
/// no refresh loop — and, on success, wins over the
/// `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` env key set; on fetch failure, or with the URL
/// unset/blank, the verifier is built from the environment exactly as
/// before, so the service always boots. Idempotent: a no-op when the
/// verifier is already built.
pub async fn init() {
    if VERIFIER.get().is_some() {
        return;
    }
    let verifier = match std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        Some(url) => {
            let issuer = env_or("PROJECT_PORTFOLIO_MANAGEMENT_TOKEN_ISSUER", DEFAULT_ISSUER);
            let audience = env_or(
                "PROJECT_PORTFOLIO_MANAGEMENT_TOKEN_AUDIENCE",
                DEFAULT_AUDIENCE,
            );
            fetch_or(url.trim(), &issuer, &audience, build_from_env()).await
        }
        None => build_from_env(),
    };
    let _ = VERIFIER.set(Arc::new(verifier));
}

/// Build a verifier by fetching the published key set from `url`
/// ([`Verifier::from_paseto_keys_url`]); on success the fetched key set
/// wins (`tracing::info!`), on any fetch/parse failure the given
/// `fallback` verifier is returned after a `tracing::warn!` — never a
/// panic, so the caller always boots. Pure dependency injection (URL,
/// issuer, audience and fallback are all passed in), so it is testable
/// against a local HTTP listener without touching the process global.
pub async fn fetch_or(url: &str, issuer: &str, audience: &str, fallback: Verifier) -> Verifier {
    match Verifier::from_paseto_keys_url(url, issuer, audience).await {
        Ok(fetched) => {
            tracing::info!(
                url,
                keys = fetched.key_count(),
                "PASETO key set fetched over HTTP; fetched key set wins over the env key set"
            );
            fetched
        }
        Err(error) => {
            tracing::warn!(
                url,
                %error,
                "PASETO key set fetch failed; falling back to the env-configured key set"
            );
            fallback
        }
    }
}

/// Whether blanket `/api/*` enforcement is on, read once from
/// `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` and cached. Off by default — see the
/// module docs and `agents/share/jwt-enforcement.md`. Mirrors
/// [`verifier`]: a process-wide `OnceLock` built from the environment.
#[must_use]
pub fn require_auth() -> bool {
    static REQUIRE_AUTH: OnceLock<bool> = OnceLock::new();
    *REQUIRE_AUTH.get_or_init(|| {
        parse_bool(&std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH").unwrap_or_default())
    })
}

/// The process-wide ABAC policy, read once from
/// `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY` / `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE` and cached
/// (restart to change). Mirrors [`require_auth`]: a process-wide
/// `OnceLock` built from the environment via [`policy_from_env`].
#[must_use]
pub fn policy() -> &'static Policy {
    static POLICY: OnceLock<Policy> = OnceLock::new();
    POLICY.get_or_init(policy_from_env)
}

/// Load the ABAC policy: `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY` (inline JSON) wins,
/// then `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE` (path to a JSON file), else the
/// built-in default policy. A present-but-unparsable policy (bad JSON,
/// unknown effect/action names, unreadable file) `tracing::warn!`s and
/// falls back to the default — the service always boots, matching the
/// key-fetch posture.
#[must_use]
pub fn policy_from_env() -> Policy {
    let source = std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            let path = std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY_FILE")
                .ok()
                .filter(|v| !v.trim().is_empty())?;
            match std::fs::read_to_string(path.trim()) {
                Ok(contents) => Some(contents),
                Err(error) => {
                    tracing::warn!(%error, %path, "ABAC policy file unreadable; using the built-in default policy");
                    None
                }
            }
        });
    match source {
        Some(json) => Policy::from_json(&json).unwrap_or_else(|error| {
            tracing::warn!(%error, "ABAC policy JSON invalid; using the built-in default policy");
            Policy::default_policy()
        }),
        None => Policy::default_policy(),
    }
}

/// Derive the request's ABAC action from its HTTP method and path (per
/// `authorization-attributes.md` §2): `GET`/`HEAD`/`OPTIONS` ⇒ `Read`;
/// `DELETE` ⇒ `Delete`; a `POST` whose path ends with a
/// [`DESTRUCTIVE_POST_SUFFIXES`] entry ⇒ `Destructive`; every other
/// `POST`/`PUT`/`PATCH` (and any unrecognised method) ⇒ `Write`.
/// (`DELETE /…/links/{id}` needs no suffix entry — the DELETE method
/// rule already covers it.)
#[must_use]
pub fn derive_action(method: &Method, path: &str) -> Action {
    // SEC-G6: normalise a trailing slash before the destructive-suffix
    // check, so `POST …/merge/` stays `Destructive` rather than being
    // downgraded to `Write` (which an `access=write` non-admin caller
    // could exploit to reach a destructive op).
    let path = path.trim_end_matches('/');
    match *method {
        Method::GET | Method::HEAD | Method::OPTIONS => Action::Read,
        Method::DELETE => Action::Delete,
        Method::POST
            if DESTRUCTIVE_POST_SUFFIXES
                .iter()
                .any(|suffix| path.ends_with(suffix)) =>
        {
            Action::Destructive
        }
        _ => Action::Write,
    }
}

/// Lenient boolean parse: `1`/`true`/`yes`/`on` (case-insensitive,
/// surrounding whitespace ignored) ⇒ `true`; everything else
/// (incl. empty) ⇒ `false`.
#[must_use]
pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Paths that stay public even when enforcement is on: health/ping, the
/// `OpenAPI` doc + Swagger UI, and the Prometheus metrics endpoint (so a
/// scraper needs no bearer token). Everything else requires a valid bearer
/// token.
fn is_public_path(path: &str) -> bool {
    path == "/_health"
        || path == "/_ping"
        || path == "/api-docs/openapi.json"
        || path.starts_with("/swagger-ui")
        || path == "/metrics.prom"
}

/// The blanket-enforcement decision: authentication, then ABAC
/// authorization. `Ok(())` ⇒ let the request through; `Err((401|403,
/// msg))` ⇒ reject. Pure: the caller passes the flag, method, path,
/// headers, verifier and policy, so it is fully unit-testable without
/// booting the app or a database.
///
/// # Errors
///
/// `401` when enforcement is on, the path is not public, and the
/// request carries no valid bearer token
/// (missing/malformed/expired/tampered). `403` when the token is valid
/// but the ABAC policy denies the derived action (the message names
/// the deciding rule, per `authorization-attributes.md` §5).
pub fn enforce(
    require_auth: bool,
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    verifier: &Verifier,
    policy: &Policy,
) -> Result<(), (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok(());
    }
    let claims = bearer_claims(headers, verifier)?;
    let decision = policy.evaluate(&claims, derive_action(method, path), ENTITY);
    if decision.allowed {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, decision.reason))
    }
}

/// Read env var `name`, treating unset/blank as absent and falling back
/// to `default`. Used for the issuer/audience so a blank value doesn't
/// override the sensible default.
fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Build the process-wide [`Verifier`] from the environment: issuer,
/// audience, and the published key set. A missing/blank/unparseable key
/// set yields an empty key set (every token rejected) so the service still
/// boots without credentials configured.
fn build_from_env() -> Verifier {
    let issuer = env_or("PROJECT_PORTFOLIO_MANAGEMENT_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or(
        "PROJECT_PORTFOLIO_MANAGEMENT_TOKEN_AUDIENCE",
        DEFAULT_AUDIENCE,
    );
    let keys = std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    Verifier::from_paseto_keys_value(&keys, &issuer, &audience)
        .unwrap_or_else(|_| empty_verifier(&issuer, &audience))
}

/// A verifier with no keys: rejects every token until a real key set is
/// configured. Infallible — an empty `keys` array always parses.
fn empty_verifier(issuer: &str, audience: &str) -> Verifier {
    let empty = serde_json::json!({ "keys": [] });
    Verifier::from_paseto_keys_value(&empty, issuer, audience).expect("empty key set always builds")
}

/// Extract and verify the bearer token from request headers. Pure (the
/// verifier is passed in), so it is unit-testable without the global.
///
/// # Errors
///
/// `401` when the `Authorization` header is missing, is not a bearer
/// token, or the token fails PASETO signature / issuer / audience / expiry
/// verification.
pub fn bearer_claims(
    headers: &HeaderMap,
    verifier: &Verifier,
) -> Result<Claims, (StatusCode, String)> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing authorization header".to_string(),
        ))?;
    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "expected bearer token".to_string(),
        ))?;
    verifier
        .verify(token.trim())
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))
}

/// A request whose bearer token passed PASETO signature / issuer /
/// audience / expiry verification. The wrapped [`Claims`] identify the
/// caller (`sub` is the user `pid`). Taking this argument makes a handler
/// require authentication.
pub struct AuthUser(pub Claims);

/// Extracting an [`AuthUser`] verifies the request's bearer token against
/// the process-wide [`verifier`]; a missing/invalid token rejects with
/// `401` before the handler runs, so the type is the "require auth" gate.
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        bearer_claims(&parts.headers, verifier()).map(AuthUser)
    }
}

/// Like [`AuthUser`], but never rejects: yields `Some(claims)` when a
/// valid bearer token is present and `None` otherwise. Handlers use it to
/// stamp the caller identity (e.g. the audit `actor`) without requiring
/// authentication on the route.
pub struct MaybeAuthUser(pub Option<Claims>);

impl MaybeAuthUser {
    /// The caller's `sub` (user `pid`) if a valid token was presented.
    #[must_use]
    pub fn actor(&self) -> Option<&str> {
        self.0.as_ref().map(|c| c.sub.as_str())
    }

    /// The verified [`Claims`] if a valid token was presented, for the
    /// record-level authorization pass ([`authorize_record`]).
    #[must_use]
    pub fn claims(&self) -> Option<&Claims> {
        self.0.as_ref()
    }
}

/// The **record-level resource attributes** of a plan for the
/// ABAC decision (`authorization-attributes.md` §9): today the
/// phase-gate `resource.stage` (PPM-3), so a deployment can write
/// e.g. "deny write past `g3_delivery` unless `access=admin`" purely
/// as policy. An item with no stage yet exposes no `stage` key.
#[must_use]
pub fn plan_resource_attrs(stage: Option<&str>) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut attrs = std::collections::BTreeMap::new();
    if let Some(stage) = stage {
        attrs.insert("stage".to_string(), vec![stage.to_string()]);
    }
    attrs
}

/// **Record-level** authorization for a handler that has loaded its
/// target: evaluate the policy with the record's resource attributes.
/// Gated on the same `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` flag
/// as the blanket guard — a no-op returning no obligations when
/// enforcement is off.
///
/// # Errors
///
/// `401` if enforcement is on but no verified claims are present
/// (fail-safe behind the guard); `403` when the policy denies the
/// action given the record's attributes.
pub fn authorize_record(
    caller: &MaybeAuthUser,
    action: Action,
    resource: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, (StatusCode, String)> {
    if !require_auth() {
        return Ok(Vec::new());
    }
    let claims = caller
        .claims()
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".to_string()))?;
    let decision = policy().evaluate_with_resource(claims, action, ENTITY, resource);
    if decision.allowed {
        Ok(decision.obligations)
    } else {
        Err((StatusCode::FORBIDDEN, decision.reason))
    }
}

/// Extracting a [`MaybeAuthUser`] never rejects: a valid token yields
/// `Some(claims)`, anything else `None`. Handlers use it to opportunistically
/// stamp the caller identity without requiring authentication.
impl<S: Send + Sync> FromRequestParts<S> for MaybeAuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(bearer_claims(&parts.headers, verifier()).ok()))
    }
}

/// DB-free, fully in-process pins for token verification and the blanket
/// `enforce` decision. A throwaway Ed25519 key mints PASETO tokens and a
/// matching key set, so the whole verification path (valid / missing /
/// non-bearer / expired / tampered / empty-keys) and the on/off/public-path
/// enforcement matrix are exercised without the auth service or a database.
#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use ed25519_dalek::SigningKey;
    use rusty_paseto::core::{
        Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4,
    };
    use sha2::{Digest, Sha256};

    /// Issuer the test tokens and verifier agree on.
    const ISSUER: &str = "authentication-service";
    /// Audience the test tokens and verifier agree on.
    const AUDIENCE: &str = "main-x-service";
    /// A throwaway Ed25519 seed, used only to mint test tokens and a
    /// matching key set in-process. Not a secret — never used in production.
    const SEED: [u8; 32] = [
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
        3, 3,
    ];

    /// Build a key set + matching `kid` from the test public key, the same
    /// way the auth service publishes it.
    fn test_keys_and_kid() -> (serde_json::Value, String) {
        let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
        let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
        let keys = serde_json::json!({
            "keys": [{
                "kty": "OKP", "crv": "Ed25519", "use": "sig",
                "kid": kid, "x": URL_SAFE_NO_PAD.encode(public),
            }]
        });
        (keys, kid)
    }

    /// Mint a signed PASETO `v4.public` token for the test identity with
    /// `kid` in the footer and `exp` set `exp_offset_secs` from a fixed
    /// `iat` (negative offsets produce an already-expired token).
    fn sign(kid: &str, exp_offset_secs: i64) -> String {
        sign_with_attrs(kid, exp_offset_secs, &[])
    }

    /// Like [`sign`], with the given ABAC subject attributes minted into
    /// the token's `attrs` claim (e.g. `&[("access", &["write"])]`).
    fn sign_with_attrs(kid: &str, exp_offset_secs: i64, attrs: &[(&str, &[&str])]) -> String {
        let iat: i64 = 1_700_000_000;
        let claims = Claims {
            sub: "11111111-1111-1111-1111-111111111111".into(),
            email: "alice@example.com".into(),
            name: "Alice".into(),
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
            exp: iat + exp_offset_secs,
            iat,
            nbf: None,
            sid: "test-sid".into(),
            scope: Vec::new(),
            roles: Vec::new(),
            attrs: attrs
                .iter()
                .map(|(key, values)| {
                    (
                        (*key).to_string(),
                        values.iter().map(ToString::to_string).collect(),
                    )
                })
                .collect(),
        };
        let keypair = SigningKey::from_bytes(&SEED).to_keypair_bytes();
        let key = Key::<64>::from(keypair);
        let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
        let payload = serde_json::to_string(&claims).expect("serialize claims");
        let footer = format!(r#"{{"kid":"{kid}"}}"#);
        let mut builder = Paseto::<V4, Public>::builder();
        builder.set_payload(Payload::from(payload.as_str()));
        builder.set_footer(Footer::from(footer.as_str()));
        builder.try_sign(&private).expect("sign")
    }

    /// Wrap a token in a `HeaderMap` with `Authorization: Bearer <token>`.
    fn bearer(token: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        h
    }

    /// A well-formed, in-date, correctly-signed token verifies and yields
    /// the expected claims.
    #[test]
    fn valid_token_yields_claims() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, 10_000_000_000);
        let claims = bearer_claims(&bearer(&token), &verifier).expect("valid token verifies");
        assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
        assert_eq!(claims.email, "alice@example.com");
    }

    /// No `Authorization` header ⇒ `401`.
    #[test]
    fn missing_header_is_401() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let err = bearer_claims(&HeaderMap::new(), &verifier).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// A non-bearer scheme (e.g. `Basic`) ⇒ `401`.
    #[test]
    fn non_bearer_header_is_401() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, "Basic abc123".parse().unwrap());
        assert_eq!(
            bearer_claims(&h, &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// A token whose `exp` is in the past ⇒ `401`.
    #[test]
    fn expired_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, -60);
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// Flipping a token character breaks PASETO verification ⇒ `401`.
    #[test]
    fn tampered_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let mut token = sign(&kid, 10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// A no-key verifier (the boot fallback) rejects even a valid token.
    #[test]
    fn empty_verifier_rejects_everything() {
        let verifier = empty_verifier(ISSUER, AUDIENCE);
        let (_, kid) = test_keys_and_kid();
        let token = sign(&kid, 10_000_000_000);
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// `parse_bool` accepts the documented truthy set and rejects the
    /// rest (including empty, `0`, and junk).
    #[test]
    fn parse_bool_truthy_and_falsy() {
        for t in ["1", "true", "TRUE", "Yes", "on", " on ", "ON"] {
            assert!(parse_bool(t), "{t:?} should parse true");
        }
        for f in ["", " ", "0", "false", "no", "off", "junk", "2"] {
            assert!(!parse_bool(f), "{f:?} should parse false");
        }
    }

    /// The default policy the enforcement tests share (what an
    /// unconfigured service uses).
    fn policy() -> Policy {
        Policy::default_policy()
    }

    /// Enforcement off ⇒ a protected path passes with no token — for
    /// reads and mutations alike (no authn and no authz when the flag
    /// is off; behaviour-neutral).
    #[test]
    fn enforce_off_allows_without_token() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        for method in [Method::GET, Method::POST, Method::DELETE] {
            assert!(
                enforce(
                    false,
                    &method,
                    "/api/projects",
                    &HeaderMap::new(),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{method} should pass with enforcement off"
            );
        }
    }

    /// Enforcement on ⇒ the public paths (health/ping, `OpenAPI`,
    /// Swagger UI, Prometheus metrics) still pass without a token.
    #[test]
    fn enforce_on_allows_public_paths() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        for path in [
            "/_health",
            "/_ping",
            "/api-docs/openapi.json",
            "/swagger-ui",
            "/swagger-ui/index.html",
            "/metrics.prom",
        ] {
            assert!(
                enforce(
                    true,
                    &Method::GET,
                    path,
                    &HeaderMap::new(),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{path} should be public"
            );
        }
    }

    /// Enforcement on, protected path, no token ⇒ `401`.
    #[test]
    fn enforce_on_protected_without_token_is_401() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let err = enforce(
            true,
            &Method::GET,
            "/api/projects",
            &HeaderMap::new(),
            &verifier,
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, valid token ⇒ a read passes.
    #[test]
    fn enforce_on_protected_with_valid_token_is_ok() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, 10_000_000_000);
        assert!(
            enforce(
                true,
                &Method::GET,
                "/api/projects",
                &bearer(&token),
                &verifier,
                &policy()
            )
            .is_ok()
        );
    }

    /// Enforcement on, protected path, expired token ⇒ `401`.
    #[test]
    fn enforce_on_protected_with_expired_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, -60);
        let err = enforce(
            true,
            &Method::GET,
            "/api/projects",
            &bearer(&token),
            &verifier,
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, tampered token ⇒ `401`.
    #[test]
    fn enforce_on_protected_with_tampered_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let mut token = sign(&kid, 10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        let err = enforce(
            true,
            &Method::GET,
            "/api/projects",
            &bearer(&token),
            &verifier,
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Action derivation (`authorization-attributes.md` §2): safe
    /// methods read; DELETE deletes; the crate's destructive named
    /// POSTs (merge today; deduplicate / bulk import when they land,
    /// their §9.8 routes ending with the same suffixes) are
    /// destructive, not write; every other POST/PUT/PATCH writes —
    /// across all four collections.
    #[test]
    fn derive_action_matrix() {
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(derive_action(&method, "/api/projects"), Action::Read);
        }
        assert_eq!(
            derive_action(&Method::DELETE, "/api/projects/123"),
            Action::Delete
        );
        // /links/{id} removal is covered by the DELETE method rule —
        // no suffix entry needed.
        assert_eq!(
            derive_action(&Method::DELETE, "/api/portfolios/123/links/456"),
            Action::Delete
        );
        for path in [
            "/api/portfolios/merge",
            "/api/projects/merge",
            "/api/products/merge",
            "/api/programs/merge",
            "/api/projects/deduplicate",
            "/api/portfolios/import", // §9.8 bulk-import route shape
        ] {
            assert_eq!(derive_action(&Method::POST, path), Action::Destructive);
        }
        // SEC-G6: a trailing slash must not downgrade a destructive POST.
        for path in [
            "/api/x/merge/",
            "/api/x/merge//",
            "/api/x/deduplicate/",
            "/api/x/import/",
        ] {
            assert_eq!(
                derive_action(&Method::POST, path),
                Action::Destructive,
                "{path}"
            );
        }
        assert_eq!(derive_action(&Method::POST, "/api/projects"), Action::Write);
        assert_eq!(
            derive_action(&Method::POST, "/api/projects/check-duplicates"),
            Action::Write
        );
        assert_eq!(
            derive_action(&Method::POST, "/api/portfolios/123/links"),
            Action::Write
        );
        assert_eq!(
            derive_action(&Method::PUT, "/api/projects/123"),
            Action::Write
        );
        assert_eq!(
            derive_action(&Method::PATCH, "/api/projects/123"),
            Action::Write
        );
        // GET on destructive-suffixed paths is still a read — only
        // POST consults the suffix list (e.g. the import-job status
        // read `GET …/import/{id}` and hypothetical `GET …/merge`).
        assert_eq!(
            derive_action(&Method::GET, "/api/portfolios/merge"),
            Action::Read
        );
    }

    /// ABAC default policy, empty `attrs` ⇒ GET allowed, POST `403`
    /// (default allow-read / deny-mutation).
    #[test]
    fn abac_empty_attrs_reads_but_cannot_write() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let token = sign_with_attrs(&kid, 10_000_000_000, &[]);
        assert!(
            enforce(
                true,
                &Method::GET,
                "/api/projects",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok()
        );
        let err = enforce(
            true,
            &Method::POST,
            "/api/projects",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    /// ABAC `access=write` ⇒ POST/PUT allowed; DELETE, the merge POST,
    /// and the (future) import POST still `403` (write is not
    /// destructive).
    #[test]
    fn abac_access_write_writes_but_not_destructive() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let token = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["write"])]);
        for method in [Method::POST, Method::PUT] {
            assert!(
                enforce(
                    true,
                    &method,
                    "/api/projects",
                    &bearer(&token),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{method} should be allowed for access=write"
            );
        }
        let delete = enforce(
            true,
            &Method::DELETE,
            "/api/projects/123",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(delete.0, StatusCode::FORBIDDEN);
        for path in ["/api/projects/merge", "/api/portfolios/import"] {
            let destructive = enforce(
                true,
                &Method::POST,
                path,
                &bearer(&token),
                &verifier,
                &policy,
            )
            .unwrap_err();
            assert_eq!(destructive.0, StatusCode::FORBIDDEN, "{path} should 403");
        }
    }

    /// ABAC `access=admin` ⇒ DELETE and the destructive named POSTs
    /// are allowed (destructive covers delete).
    #[test]
    fn abac_access_admin_allows_destructive() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let token = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["admin"])]);
        assert!(
            enforce(
                true,
                &Method::DELETE,
                "/api/projects/123",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok()
        );
        for path in [
            "/api/projects/merge",
            "/api/projects/deduplicate",
            "/api/portfolios/import",
        ] {
            assert!(
                enforce(
                    true,
                    &Method::POST,
                    path,
                    &bearer(&token),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{path} should be allowed for access=admin"
            );
        }
    }

    /// ABAC `svc=true` (machine peer) ⇒ everything is allowed.
    #[test]
    fn abac_svc_true_allows_everything() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let token = sign_with_attrs(&kid, 10_000_000_000, &[("svc", &["true"])]);
        for (method, path) in [
            (Method::GET, "/api/projects"),
            (Method::POST, "/api/projects"),
            (Method::PUT, "/api/projects/123"),
            (Method::DELETE, "/api/projects/123"),
            (Method::POST, "/api/projects/merge"),
            (Method::POST, "/api/portfolios/import"),
        ] {
            assert!(
                enforce(true, &method, path, &bearer(&token), &verifier, &policy).is_ok(),
                "{method} {path} should be allowed for svc=true"
            );
        }
    }

    /// A configured deny rule ahead of an allow rule wins
    /// (first-match-wins pin, through the guard).
    #[test]
    fn abac_configured_deny_beats_later_allow() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "deny",  "actions": ["write"], "when": { "dept": ["pmo"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
            ] }"#,
        )
        .expect("policy parses");
        let denied = sign_with_attrs(
            &kid,
            10_000_000_000,
            &[("access", &["write"]), ("dept", &["pmo"])],
        );
        let err = enforce(
            true,
            &Method::POST,
            "/api/projects",
            &bearer(&denied),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
        let allowed = sign_with_attrs(&kid, 10_000_000_000, &[("access", &["write"])]);
        assert!(
            enforce(
                true,
                &Method::POST,
                "/api/projects",
                &bearer(&allowed),
                &verifier,
                &policy
            )
            .is_ok()
        );
    }

    /// 401 vs 403: missing/bad credential is `401`; a valid credential
    /// the policy denies is `403` with the deciding-rule reason
    /// (`"default deny"` when no rule matched).
    #[test]
    fn abac_401_versus_403_distinction() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let policy = policy();
        let no_token = enforce(
            true,
            &Method::POST,
            "/api/projects",
            &HeaderMap::new(),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(no_token.0, StatusCode::UNAUTHORIZED);
        let token = sign_with_attrs(&kid, 10_000_000_000, &[]);
        let denied = enforce(
            true,
            &Method::POST,
            "/api/projects",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(denied.0, StatusCode::FORBIDDEN);
        assert_eq!(denied.1, "default deny");
    }

    /// `policy_from_env` never breaks boot: bad policy JSON falls back
    /// to the built-in default policy. (Pinned on the pure fallback
    /// expression rather than by mutating the process-global env,
    /// which other tests share.)
    #[test]
    fn policy_bad_json_falls_back_to_default() {
        assert!(Policy::from_json("{ not json").is_err());
        assert_eq!(
            Policy::from_json("{ not json").unwrap_or_else(|_| Policy::default_policy()),
            Policy::default_policy()
        );
    }

    /// Serve `keys` as the key-set JSON from a local ephemeral-port HTTP
    /// listener (the auth service's `/.well-known/paseto-keys` stand-in)
    /// and return the URL to fetch it from.
    async fn serve_keys(keys: serde_json::Value) -> String {
        let app = axum::Router::new().route(
            "/.well-known/paseto-keys",
            axum::routing::get(move || {
                let keys = keys.clone();
                async move { axum::Json(keys) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve key set");
        });
        format!("http://{addr}/.well-known/paseto-keys")
    }

    /// Boot-time fetch happy path: `fetch_or` against a local listener
    /// serving a valid key set builds the verifier from the **fetched**
    /// keys — a token signed by the served key verifies even though the
    /// fallback verifier has no keys (the fetched key set wins).
    #[tokio::test]
    async fn fetch_or_fetched_key_set_wins() {
        let (keys, kid) = test_keys_and_kid();
        let url = serve_keys(keys).await;
        let verifier = fetch_or(&url, ISSUER, AUDIENCE, empty_verifier(ISSUER, AUDIENCE)).await;
        assert_eq!(verifier.key_count(), 1);
        let token = sign(&kid, 10_000_000_000);
        let claims =
            bearer_claims(&bearer(&token), &verifier).expect("token signed by fetched key");
        assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
    }

    /// Boot-time fetch fallback: a fast-failing URL (nothing listens on
    /// port 1) makes `fetch_or` return the env-style fallback verifier —
    /// no panic, and tokens signed by the fallback key set still verify,
    /// so the service always boots.
    #[tokio::test]
    async fn fetch_or_unreachable_url_falls_back() {
        let (keys, kid) = test_keys_and_kid();
        let fallback = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let verifier = fetch_or("http://127.0.0.1:1/", ISSUER, AUDIENCE, fallback).await;
        assert_eq!(verifier.key_count(), 1);
        let token = sign(&kid, 10_000_000_000);
        assert!(bearer_claims(&bearer(&token), &verifier).is_ok());
    }
}
