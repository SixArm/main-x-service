//! Bearer-token authentication for the REST surface.
//!
//! [`AuthUser`] is an Axum extractor that pulls `Authorization: Bearer
//! <paseto>`, verifies the PASETO `v4.public` (Ed25519) signature and
//! claims against the authentication-service published key set (carried
//! in [`AppState::verifier`]), and yields the verified [`Claims`].
//! Verification is stateless and offline — no database hit, no
//! introspection call — so any handler can require authentication by
//! taking an `AuthUser` argument. See
//! `agents/share/authentication-sessions.md` for the family-wide design
//! (cookie sessions + short-lived PASETO v4.public cross-service tokens;
//! this replaces the earlier RS256-JWT + JWKS model).
//!
//! ## Blanket enforcement (spec §13 T-1b)
//!
//! When `WORKER_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
//! case-insensitive), the [`enforce`] decision — wired as an Axum
//! middleware layer via [`apply_enforcement`] on **both** router
//! surfaces (`create_router` and the loco router in
//! `App::after_routes`) — requires a valid PASETO `v4.public` bearer
//! token on every route except the public allow-list in
//! [`PUBLIC_PATHS`] / [`PUBLIC_PATH_PREFIXES`] (health probes, the
//! `OpenAPI` document + Swagger UI, and the Prometheus scrape endpoint).
//! It is **off by default**: unset/blank/`0`/junk ⇒ today's behaviour,
//! where authentication is opt-in per handler. The flag is read **once,
//! at router construction** ([`require_auth_from_env`]) — changing it
//! requires a process restart. Activation is an operations decision once
//! the SSO token flow is live; the family-wide contract is
//! `agents/share/jwt-enforcement.md`.
//!
//! ## Authorization (ABAC)
//!
//! Inside the same guard — so it applies only when `WORKER_REQUIRE_AUTH`
//! is on — a verified token is further checked against an
//! **attribute-based access control** policy per
//! `agents/share/authorization-attributes.md`: the request's action is
//! derived from the HTTP method plus this crate's destructive named
//! POSTs ([`DESTRUCTIVE_POST_SUFFIXES`] — `/api/workers/merge`,
//! `/api/workers/deduplicate` today), and the shared engine in the
//! `authentication-verifier` crate evaluates the policy over the
//! token's `attrs` claim. The policy is read once at router
//! construction ([`policy_from_env`]) from `WORKER_ABAC_POLICY` (inline
//! JSON) or `WORKER_ABAC_POLICY_FILE` (path); unset or unparsable ⇒ the
//! built-in default policy (`svc=true` ⇒ everything; `access=admin` ⇒
//! destructive+write; `access=write` ⇒ write; otherwise read-only) —
//! the service always boots. **401** = missing/bad credential; **403**
//! = valid credential, policy denied (body carries the deciding rule).

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use authentication_verifier::{Action, Claims, Policy, Verifier};
use axum::extract::{FromRef, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::{Json, Router};

use super::state::AppState;
use crate::models::worker::Worker;

/// The resource entity this crate guards, as seen by ABAC policies
/// (the `entity` pseudo-attribute in rule `when` clauses).
pub const ENTITY: &str = "worker";

/// Path suffixes of this crate's **destructive named POSTs** (per
/// `authorization-attributes.md` §2): record merge and batch
/// deduplicate today, bulk import when the bulk-import feature lands.
/// A POST whose path ends with one of these derives
/// [`Action::Destructive`] instead of [`Action::Write`].
pub const DESTRUCTIVE_POST_SUFFIXES: [&str; 4] = ["/merge", "/deduplicate", "/import", "/erase"];

/// Exact-match paths that stay public even when blanket enforcement is
/// on: loco's default health probes (`/_health`, `/_ping`), this crate's
/// own health endpoint (`/api/health` — orchestration probes carry no
/// token), the served `OpenAPI` document, and the Prometheus scrape
/// endpoint (`/metrics.prom` — scrapers carry no token). Everything else
/// — the whole `/api` surface and the `/fhir` surface (worker PII) —
/// requires a valid bearer token when enforcement is on.
pub const PUBLIC_PATHS: [&str; 5] = [
    "/_health",
    "/_ping",
    "/api/health",
    "/api-docs/openapi.json",
    "/metrics.prom",
];

/// Prefix-match public paths: the Swagger UI page and its assets
/// (`/swagger-ui`, `/swagger-ui/…`).
pub const PUBLIC_PATH_PREFIXES: [&str; 1] = ["/swagger-ui"];

/// Whether `path` is on the public allow-list ([`PUBLIC_PATHS`] exact or
/// [`PUBLIC_PATH_PREFIXES`] prefix match).
fn is_public_path(path: &str) -> bool {
    PUBLIC_PATHS.contains(&path) || PUBLIC_PATH_PREFIXES.iter().any(|p| path.starts_with(p))
}

/// Lenient boolean parse for the enforcement flag: `1`/`true`/`yes`/`on`
/// (case-insensitive, surrounding whitespace ignored) ⇒ `true`;
/// everything else (incl. empty, `0`, junk) ⇒ `false`.
#[must_use]
pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Read the blanket-enforcement flag from `WORKER_REQUIRE_AUTH`
/// (default **off**: unset/blank/`0`/junk ⇒ `false`). Called once at
/// router construction — changing the flag requires a restart.
#[must_use]
pub fn require_auth_from_env() -> bool {
    parse_bool(&std::env::var("WORKER_REQUIRE_AUTH").unwrap_or_default())
}

/// Derive the request's ABAC action from its HTTP method and path (per
/// `authorization-attributes.md` §2): `GET`/`HEAD`/`OPTIONS` ⇒ `Read`;
/// `DELETE` ⇒ `Delete`; a `POST` whose path ends with a
/// [`DESTRUCTIVE_POST_SUFFIXES`] entry ⇒ `Destructive`; every other
/// `POST`/`PUT`/`PATCH` (and any unrecognised method) ⇒ `Write`.
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

/// Load the ABAC policy: `WORKER_ABAC_POLICY` (inline JSON) wins, then
/// `WORKER_ABAC_POLICY_FILE` (path to a JSON file), else the built-in
/// default policy. A present-but-unparsable policy (bad JSON, unknown
/// effect/action names, unreadable file) `tracing::warn!`s and falls
/// back to the default — the service always boots, matching the
/// key-fetch posture. Read once at router construction; restart to
/// change.
#[must_use]
pub fn policy_from_env() -> Policy {
    let source = std::env::var("WORKER_ABAC_POLICY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            let path = std::env::var("WORKER_ABAC_POLICY_FILE")
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

/// Layer the blanket-enforcement middleware onto a finished router. The
/// flag, verifier, and ABAC policy are captured **at construction**
/// (restart to change); when the flag is off the middleware is a
/// near-noop, so both router surfaces wire it unconditionally and
/// `WORKER_REQUIRE_AUTH` is the only switch. Applied beneath the CORS
/// layer so preflight `OPTIONS` requests are answered by CORS before
/// enforcement runs.
pub fn apply_enforcement(
    router: Router,
    require_auth: bool,
    verifier: Arc<Verifier>,
    policy: Arc<Policy>,
) -> Router {
    router.layer(axum::middleware::from_fn(
        move |req: Request, next: Next| {
            let verifier = Arc::clone(&verifier);
            let policy = Arc::clone(&policy);
            async move {
                match enforce(
                    require_auth,
                    req.method(),
                    req.uri().path(),
                    req.headers(),
                    &verifier,
                    &policy,
                ) {
                    Ok(()) => next.run(req).await,
                    Err(reject) => reject.into_response(),
                }
            }
        },
    ))
}

/// Extract and verify the bearer token from request headers. Pure (the
/// verifier is passed in), so it is unit-testable without an [`AppState`]
/// or a database.
///
/// # Errors
///
/// `401` when the `Authorization` header is missing, is not a bearer
/// token, or the token fails PASETO signature / issuer / audience /
/// expiry verification.
pub fn bearer_claims(
    headers: &HeaderMap,
    verifier: &Verifier,
) -> Result<Claims, (StatusCode, String)> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
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

/// A request whose bearer token passed PASETO `v4.public` signature /
/// issuer / audience / expiry verification. The wrapped [`Claims`]
/// identify the caller (`sub` is the user `pid`).
pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = AppState::from_ref(state);
        bearer_claims(&parts.headers, &app.verifier).map(AuthUser)
    }
}

/// The caller's verified claims **when present** — `Some(claims)` for a
/// valid bearer, `None` otherwise. Never rejects, so a handler can run a
/// **record-level** authorization pass ([`authorize_record`]) only when
/// enforcement is on (behind the blanket guard a token is guaranteed).
pub struct MaybeAuthUser(pub Option<Claims>);

impl MaybeAuthUser {
    /// The verified claims if a valid token was presented.
    #[must_use]
    pub fn claims(&self) -> Option<&Claims> {
        self.0.as_ref()
    }
}

impl<S> FromRequestParts<S> for MaybeAuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = AppState::from_ref(state);
        Ok(MaybeAuthUser(
            bearer_claims(&parts.headers, &app.verifier).ok(),
        ))
    }
}

/// Whether blanket enforcement is on, cached from `WORKER_REQUIRE_AUTH`
/// on first use (mirrors what [`EnforcementState`] snapshots at router
/// construction). Used by [`authorize_record`].
#[must_use]
pub fn require_auth() -> bool {
    static FLAG: OnceLock<bool> = OnceLock::new();
    *FLAG.get_or_init(require_auth_from_env)
}

/// The ABAC policy for handler-level record checks, cached from the
/// environment on first use ([`policy_from_env`]). Read once; restart to
/// change.
#[must_use]
pub fn policy() -> &'static Policy {
    static POLICY: OnceLock<Policy> = OnceLock::new();
    POLICY.get_or_init(policy_from_env)
}

/// Derive the **record-level resource attributes** of a stored worker
/// for the ABAC decision (`authorization-attributes.md` §9):
/// `resource.active` / `resource.deceased` (`true`/`false`) and
/// `resource.managing_org` (the org `pid`, when set). A deployment can
/// then write e.g. "deny write on an inactive worker's record unless
/// `access=admin`". No schema change — existing fields.
#[must_use]
pub fn worker_resource_attrs(worker: &Worker) -> BTreeMap<String, Vec<String>> {
    let mut attrs = BTreeMap::new();
    attrs.insert("active".to_string(), vec![worker.active.to_string()]);
    attrs.insert("deceased".to_string(), vec![worker.deceased.to_string()]);
    if let Some(org) = worker.managing_organization {
        attrs.insert("managing_org".to_string(), vec![org.to_string()]);
    }
    attrs
}

/// Environment attributes for the current request, for the `env.*`
/// policy namespace (`authorization-attributes.md` §10): `env.hour`
/// (UTC 0–23) and `env.after_hours` (true outside 08:00–17:59 UTC),
/// derived at the service edge so the engine stays deterministic.
#[must_use]
pub fn request_env_attrs() -> BTreeMap<String, Vec<String>> {
    use chrono::Timelike;
    env_attrs_at(chrono::Utc::now().hour())
}

/// Pure derivation of [`request_env_attrs`] for a given UTC `hour`.
#[must_use]
fn env_attrs_at(hour: u32) -> BTreeMap<String, Vec<String>> {
    let after_hours = !(8..18).contains(&hour);
    let mut env = BTreeMap::new();
    env.insert("hour".to_string(), vec![hour.to_string()]);
    env.insert("after_hours".to_string(), vec![after_hours.to_string()]);
    env
}

/// **Record-level** authorization for a handler that has loaded the
/// target worker: evaluate the policy with the record's resource
/// attributes ([`worker_resource_attrs`]) and the request's environment
/// attributes ([`request_env_attrs`]). Gated on `WORKER_REQUIRE_AUTH`
/// (a no-op when off). On an **allow**, returns the decision's
/// obligations (e.g. `["mask"]`) for the handler to honour.
///
/// # Errors
///
/// `401` if enforcement is on but no verified claims are present. `403`
/// if the policy denies the action (the message names the deciding rule).
pub fn authorize_record(
    caller: &MaybeAuthUser,
    action: Action,
    resource: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, (StatusCode, String)> {
    if !require_auth() {
        return Ok(Vec::new());
    }
    let claims = caller
        .claims()
        .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token".to_string()))?;
    let decision =
        policy().evaluate_with_context(claims, action, ENTITY, resource, &request_env_attrs());
    if decision.allowed {
        Ok(decision.obligations)
    } else {
        Err((StatusCode::FORBIDDEN, decision.reason))
    }
}

/// `GET /api/whoami` — echo the verified claims of the bearer token.
/// Returns `401` when the token is missing, malformed, or fails
/// verification. Useful for confirming peer PASETO verification end to
/// end.
#[utoipa::path(
    get,
    path = "/api/whoami",
    tag = "auth",
    responses(
        (status = 200, description = "Verified token claims"),
        (status = 401, description = "Missing or invalid bearer token"),
    ),
    security(("bearer" = [])),
)]
pub async fn whoami(AuthUser(claims): AuthUser) -> impl IntoResponse {
    Json(claims)
}

/// DB-free, fully in-process pins for the bearer-verification path. A
/// throwaway Ed25519 key mints PASETO `v4.public` tokens and a matching
/// key set, so valid / missing / non-bearer / expired / tampered /
/// no-key cases are exercised without the auth service or a network.
#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use ed25519_dalek::SigningKey;
    use rusty_paseto::core::{
        Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4,
    };

    /// Issuer the test tokens and verifier agree on.
    const ISSUER: &str = "authentication-service";
    /// Audience the test tokens and verifier agree on.
    const AUDIENCE: &str = "main-x-service";
    /// Key id stamped in the token footer and published in the key set.
    const KID: &str = "test-key-1";
    /// A throwaway Ed25519 seed, used only to mint test tokens and a
    /// matching key set in-process. Not a secret — never used in
    /// production.
    const SEED: [u8; 32] = [
        7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        7, 7,
    ];

    /// Build a key set from the test public key, the same way the auth
    /// service publishes it at `/.well-known/paseto-keys`.
    fn test_keys() -> serde_json::Value {
        let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
        serde_json::json!({
            "keys": [{
                "kty": "OKP", "crv": "Ed25519", "use": "sig",
                "kid": KID, "x": URL_SAFE_NO_PAD.encode(public),
            }]
        })
    }

    /// Mint a signed PASETO `v4.public` token for the test identity with
    /// [`KID`] in the footer and `exp` set `exp_offset_secs` from a fixed
    /// `iat` (negative offsets produce an already-expired token).
    fn sign(exp_offset_secs: i64) -> String {
        sign_with_attrs(exp_offset_secs, &[])
    }

    /// Like [`sign`], with the given ABAC subject attributes minted into
    /// the token's `attrs` claim (e.g. `&[("access", &["write"])]`).
    fn sign_with_attrs(exp_offset_secs: i64, attrs: &[(&str, &[&str])]) -> String {
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
        let footer = format!(r#"{{"kid":"{KID}"}}"#);
        let mut builder = Paseto::<V4, Public>::builder();
        builder.set_payload(Payload::from(payload.as_str()));
        builder.set_footer(Footer::from(footer.as_str()));
        builder.try_sign(&private).expect("sign")
    }

    /// The verifier the tests share, built from the in-process key set.
    fn verifier() -> Verifier {
        Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).expect("key set builds")
    }

    /// Wrap a token in a `HeaderMap` with `Authorization: Bearer <token>`.
    fn bearer(token: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {token}").parse().unwrap(),
        );
        h
    }

    #[test]
    fn test_valid_token_yields_claims() {
        let token = sign(10_000_000_000);
        let claims = bearer_claims(&bearer(&token), &verifier()).expect("valid token verifies");
        assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
        assert_eq!(claims.email, "alice@example.com");
        assert_eq!(claims.iss, ISSUER);
        assert_eq!(claims.aud, AUDIENCE);
    }

    #[test]
    fn test_missing_header_is_401() {
        let err = bearer_claims(&HeaderMap::new(), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_non_bearer_header_is_401() {
        let mut h = HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            "Basic abc123".parse().unwrap(),
        );
        assert_eq!(
            bearer_claims(&h, &verifier()).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn test_expired_token_is_401() {
        let token = sign(-60);
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier()).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn test_tampered_token_is_401() {
        let mut token = sign(10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier()).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn test_empty_key_set_rejects_valid_token() {
        let empty = serde_json::json!({ "keys": [] });
        let no_keys =
            Verifier::from_paseto_keys_value(&empty, ISSUER, AUDIENCE).expect("empty set builds");
        let token = sign(10_000_000_000);
        assert_eq!(
            bearer_claims(&bearer(&token), &no_keys).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// `parse_bool` accepts the documented truthy set and rejects the
    /// rest (including empty, `0`, and junk) — the `WORKER_REQUIRE_AUTH`
    /// flag semantics from `agents/share/jwt-enforcement.md`.
    #[test]
    fn test_parse_bool_truthy_and_falsy() {
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

    /// Enforcement off ⇒ a protected path passes with no token (today's
    /// default behaviour is preserved) — for reads and mutations alike
    /// (no authn and no authz when the flag is off).
    #[test]
    fn test_enforce_off_allows_protected_without_token() {
        let (verifier, policy) = (verifier(), policy());
        for method in [Method::GET, Method::POST, Method::DELETE] {
            assert!(
                enforce(
                    false,
                    &method,
                    "/api/workers",
                    &HeaderMap::new(),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{method} should pass with enforcement off"
            );
        }
    }

    /// Enforcement on ⇒ every allow-listed public path still passes
    /// without a token.
    #[test]
    fn test_enforce_on_allows_public_paths() {
        let (verifier, policy) = (verifier(), policy());
        for path in PUBLIC_PATHS {
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
        for path in ["/swagger-ui", "/swagger-ui/index.html"] {
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
    fn test_enforce_on_protected_without_token_is_401() {
        let err = enforce(
            true,
            &Method::GET,
            "/api/workers",
            &HeaderMap::new(),
            &verifier(),
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, valid token ⇒ a read passes.
    #[test]
    fn test_enforce_on_protected_with_valid_token_is_ok() {
        let token = sign(10_000_000_000);
        assert!(
            enforce(
                true,
                &Method::GET,
                "/api/workers",
                &bearer(&token),
                &verifier(),
                &policy()
            )
            .is_ok()
        );
    }

    /// Enforcement on, protected path, expired token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_with_expired_token_is_401() {
        let token = sign(-60);
        let err = enforce(
            true,
            &Method::GET,
            "/api/workers",
            &bearer(&token),
            &verifier(),
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, tampered token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_with_tampered_token_is_401() {
        let mut token = sign(10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        let err = enforce(
            true,
            &Method::GET,
            "/api/workers",
            &bearer(&token),
            &verifier(),
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// The FHIR surface is protected too — it serves worker PII, so it
    /// is deliberately not on the allow-list.
    #[test]
    fn test_enforce_on_fhir_without_token_is_401() {
        let err = enforce(
            true,
            &Method::GET,
            "/fhir/Practitioner",
            &HeaderMap::new(),
            &verifier(),
            &policy(),
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Action derivation (`authorization-attributes.md` §2): safe
    /// methods read; DELETE deletes; the crate's destructive named
    /// POSTs (merge / deduplicate / import) are destructive, not
    /// write; every other POST/PUT/PATCH writes.
    #[test]
    fn test_derive_action_matrix() {
        for method in [Method::GET, Method::HEAD, Method::OPTIONS] {
            assert_eq!(derive_action(&method, "/api/workers"), Action::Read);
        }
        assert_eq!(
            derive_action(&Method::DELETE, "/api/workers/1"),
            Action::Delete
        );
        for path in [
            "/api/workers/merge",
            "/api/workers/deduplicate",
            "/api/workers/import",
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
        assert_eq!(derive_action(&Method::POST, "/api/workers"), Action::Write);
        assert_eq!(
            derive_action(&Method::POST, "/api/workers/check-duplicates"),
            Action::Write
        );
        assert_eq!(derive_action(&Method::PUT, "/api/workers/1"), Action::Write);
        assert_eq!(
            derive_action(&Method::PATCH, "/api/workers/1"),
            Action::Write
        );
        // GET on a destructive-suffixed path is still a read — only
        // POST consults the suffix list.
        assert_eq!(
            derive_action(&Method::GET, "/api/workers/merge"),
            Action::Read
        );
    }

    /// ABAC default policy, empty `attrs` ⇒ GET allowed, POST `403`
    /// (default allow-read / deny-mutation).
    #[test]
    fn test_abac_empty_attrs_reads_but_cannot_write() {
        let (verifier, policy) = (verifier(), policy());
        let token = sign_with_attrs(10_000_000_000, &[]);
        assert!(
            enforce(
                true,
                &Method::GET,
                "/api/workers",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok()
        );
        let err = enforce(
            true,
            &Method::POST,
            "/api/workers",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
    }

    /// ABAC `access=write` ⇒ POST/PUT allowed; DELETE and the merge
    /// POST still `403` (write is not destructive).
    #[test]
    fn test_abac_access_write_writes_but_not_destructive() {
        let (verifier, policy) = (verifier(), policy());
        let token = sign_with_attrs(10_000_000_000, &[("access", &["write"])]);
        for method in [Method::POST, Method::PUT] {
            assert!(
                enforce(
                    true,
                    &method,
                    "/api/workers",
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
            "/api/workers/1",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(delete.0, StatusCode::FORBIDDEN);
        let merge = enforce(
            true,
            &Method::POST,
            "/api/workers/merge",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(merge.0, StatusCode::FORBIDDEN);
    }

    /// ABAC `access=admin` ⇒ DELETE and the destructive named POSTs
    /// are allowed (destructive covers delete).
    #[test]
    fn test_abac_access_admin_allows_destructive() {
        let (verifier, policy) = (verifier(), policy());
        let token = sign_with_attrs(10_000_000_000, &[("access", &["admin"])]);
        assert!(
            enforce(
                true,
                &Method::DELETE,
                "/api/workers/1",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok()
        );
        for path in ["/api/workers/merge", "/api/workers/deduplicate"] {
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
    fn test_abac_svc_true_allows_everything() {
        let (verifier, policy) = (verifier(), policy());
        let token = sign_with_attrs(10_000_000_000, &[("svc", &["true"])]);
        for (method, path) in [
            (Method::GET, "/api/workers"),
            (Method::POST, "/api/workers"),
            (Method::PUT, "/api/workers/1"),
            (Method::DELETE, "/api/workers/1"),
            (Method::POST, "/api/workers/merge"),
            (Method::POST, "/api/workers/deduplicate"),
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
    fn test_abac_configured_deny_beats_later_allow() {
        let verifier = verifier();
        let policy = Policy::from_json(
            r#"{ "rules": [
                { "effect": "deny",  "actions": ["write"], "when": { "dept": ["oncology"] } },
                { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
            ] }"#,
        )
        .expect("policy parses");
        let denied = sign_with_attrs(
            10_000_000_000,
            &[("access", &["write"]), ("dept", &["oncology"])],
        );
        let err = enforce(
            true,
            &Method::POST,
            "/api/workers",
            &bearer(&denied),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(err.0, StatusCode::FORBIDDEN);
        let allowed = sign_with_attrs(10_000_000_000, &[("access", &["write"])]);
        assert!(
            enforce(
                true,
                &Method::POST,
                "/api/workers",
                &bearer(&allowed),
                &verifier,
                &policy
            )
            .is_ok()
        );
    }

    /// 401 vs 403: missing/bad credential is `401`; a valid credential
    /// the policy denies is `403` with the deciding-rule reason.
    #[test]
    fn test_abac_401_versus_403_distinction() {
        let (verifier, policy) = (verifier(), policy());
        let no_token = enforce(
            true,
            &Method::POST,
            "/api/workers",
            &HeaderMap::new(),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(no_token.0, StatusCode::UNAUTHORIZED);
        let token = sign_with_attrs(10_000_000_000, &[]);
        let denied = enforce(
            true,
            &Method::POST,
            "/api/workers",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(denied.0, StatusCode::FORBIDDEN);
        assert_eq!(denied.1, "default deny");
    }

    /// `policy_from_env` never breaks boot: bad inline JSON falls back
    /// to the built-in default policy (the pure fallback path taken on
    /// parse failure is `Policy::default_policy()`).
    #[test]
    fn test_policy_from_env_bad_json_falls_back_to_default() {
        assert!(Policy::from_json("{ not json").is_err());
        assert_eq!(
            Policy::from_json("{ not json").unwrap_or_else(|_| Policy::default_policy()),
            Policy::default_policy()
        );
    }

    // ---- Boot-time key-set fetch (`WORKER_PASETO_KEYS_URL`) ----

    use crate::api::rest::state::verifier_from_url_or_env;

    /// Serve `test_keys()` from a local ephemeral-port HTTP listener,
    /// the way the auth service publishes `/.well-known/paseto-keys`.
    /// Returns the full key-set URL; the server task lives until the
    /// test process exits.
    async fn serve_test_keys() -> String {
        let router = Router::new().route(
            "/.well-known/paseto-keys",
            axum::routing::get(|| async { Json(test_keys()) }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, router).await.expect("serve keys");
        });
        format!("http://{addr}/.well-known/paseto-keys")
    }

    /// URL unset ⇒ the env-key-set path: with no `WORKER_PASETO_KEYS`
    /// pointing at the test key, a token signed by it is rejected (and
    /// the builder never panics).
    #[tokio::test]
    async fn test_fetch_builder_without_url_uses_env_path() {
        let v = verifier_from_url_or_env(None, ISSUER, AUDIENCE).await;
        let token = sign(10_000_000_000);
        assert_eq!(
            bearer_claims(&bearer(&token), &v).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// URL set and reachable ⇒ the fetched key set wins: a token signed
    /// by the served key verifies end to end.
    #[tokio::test]
    async fn test_fetch_builder_fetches_key_set_from_url() {
        let url = serve_test_keys().await;
        let v = verifier_from_url_or_env(Some(&url), ISSUER, AUDIENCE).await;
        assert_eq!(v.key_count(), 1);
        let token = sign(10_000_000_000);
        let claims = bearer_claims(&bearer(&token), &v).expect("token from fetched key verifies");
        assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
        assert_eq!(claims.iss, ISSUER);
        assert_eq!(claims.aud, AUDIENCE);
    }

    /// URL set but unreachable (bind an ephemeral port, note it, drop
    /// the listener) ⇒ the builder falls back to the env-key-set path
    /// without panicking, so the service always boots: the test-key
    /// token is rejected exactly as on the env path.
    #[tokio::test]
    async fn test_fetch_builder_falls_back_on_fetch_failure() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        drop(listener);
        let url = format!("http://{addr}/.well-known/paseto-keys");
        let v = verifier_from_url_or_env(Some(&url), ISSUER, AUDIENCE).await;
        let token = sign(10_000_000_000);
        assert_eq!(
            bearer_claims(&bearer(&token), &v).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// The record-level resource-attribute derivation maps a worker's
    /// coarse status to the `resource.*` tokens a policy matches (§9).
    #[test]
    fn worker_resource_attrs_maps_status_fields() {
        use crate::models::Gender;
        use crate::models::worker::HumanName;
        let mut worker = Worker::new(
            HumanName {
                use_type: None,
                family: "Smith".to_string(),
                given: vec!["John".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Male,
        );
        worker.active = false;
        worker.deceased = false;
        let org = uuid::Uuid::new_v4();
        worker.managing_organization = Some(org);

        let attrs = worker_resource_attrs(&worker);
        assert_eq!(attrs["active"], vec!["false".to_string()]);
        assert_eq!(attrs["deceased"], vec!["false".to_string()]);
        assert_eq!(attrs["managing_org"], vec![org.to_string()]);

        worker.managing_organization = None;
        assert!(!worker_resource_attrs(&worker).contains_key("managing_org"));
    }

    /// The environment-attribute derivation flags working vs after hours.
    #[test]
    fn env_attrs_at_flags_after_hours() {
        for hour in 8..18 {
            assert_eq!(env_attrs_at(hour)["after_hours"], vec!["false".to_string()]);
        }
        for hour in [0, 7, 18, 23] {
            assert_eq!(env_attrs_at(hour)["after_hours"], vec!["true".to_string()]);
        }
    }
}
