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
//! ## Blanket enforcement
//!
//! When `PLACE_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
//! case-insensitive), the [`enforce`] decision — wired as an Axum
//! middleware layer on both router surfaces (`create_router` and the
//! loco router in `src/app.rs`) — requires a valid bearer token on
//! every route except the public allow-list ([`PUBLIC_PATHS`] +
//! [`PUBLIC_PATH_PREFIXES`]: health, OpenAPI/Swagger docs, Prometheus
//! metrics). It is **off by default**: unset/blank/`0`/junk ⇒ today's
//! behaviour, where the extractor is opt-in per handler and
//! `GET /api/whoami` proves end-to-end verification. The flag is read
//! once at router construction ([`EnforcementState::from_app_state`]),
//! so changing it requires a restart. Activation is an operations
//! decision once the SSO token flow is live; see
//! `agents/share/authentication-sessions.md` and
//! `agents/share/jwt-enforcement.md` for the family-wide contract.
//!
//! ## Authorization (ABAC)
//!
//! Inside the same guard — so it applies only when `PLACE_REQUIRE_AUTH`
//! is on — a verified token is further checked against an
//! **attribute-based access control** policy per
//! `agents/share/authorization-attributes.md`: the request's action is
//! derived from the HTTP method plus this crate's destructive named
//! POSTs ([`DESTRUCTIVE_POST_SUFFIXES`]), and the shared engine in the
//! `authentication-verifier` crate evaluates the policy over the
//! token's `attrs` claim. The policy is read once at router
//! construction ([`policy_from_env`]) from `PLACE_ABAC_POLICY` (inline
//! JSON) or `PLACE_ABAC_POLICY_FILE` (path); unset or unparsable ⇒ the
//! built-in default policy (`svc=true` ⇒ everything; `access=admin` ⇒
//! destructive+write; `access=write` ⇒ write; otherwise read-only) —
//! the service always boots. **401** = missing/bad credential; **403**
//! = valid credential, policy denied (body carries the deciding rule).

use std::sync::OnceLock;

use authentication_verifier::{
    Action, Claims, Policy, ReloadablePolicy, ReloadableVerifier, Verifier,
};
use axum::Json;
use axum::extract::{FromRef, FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::state::AppState;

/// The resource entity this crate guards, as seen by ABAC policies
/// (the `entity` pseudo-attribute in rule `when` clauses).
pub const ENTITY: &str = "place";

/// Path suffixes of this crate's **destructive named POSTs** (per
/// `authorization-attributes.md` §2): record merge and batch
/// deduplicate today, bulk import when T-10 lands. A POST whose path
/// ends with one of these derives [`Action::Destructive`] instead of
/// [`Action::Write`].
pub const DESTRUCTIVE_POST_SUFFIXES: [&str; 3] = ["/merge", "/deduplicate", "/import"];

/// Exact paths that stay public even when blanket enforcement is on:
/// the health probes (`/api/health` on this crate's surface, plus
/// loco's default `/_health` / `/_ping`), the raw `OpenAPI` document,
/// and the root-mounted Prometheus scrape path (outside `/api`, so a
/// scraper needs no bearer token). Everything else requires a valid
/// bearer token when `PLACE_REQUIRE_AUTH` is on.
pub const PUBLIC_PATHS: [&str; 5] = [
    "/api/health",
    "/_health",
    "/_ping",
    "/api-docs/openapi.json",
    "/metrics.prom",
];

/// Path prefixes that stay public even when blanket enforcement is on:
/// the Swagger UI and its static assets.
pub const PUBLIC_PATH_PREFIXES: [&str; 1] = ["/swagger-ui"];

/// Whether `path` is on the public allow-list ([`PUBLIC_PATHS`] +
/// [`PUBLIC_PATH_PREFIXES`]).
fn is_public_path(path: &str) -> bool {
    PUBLIC_PATHS.contains(&path) || PUBLIC_PATH_PREFIXES.iter().any(|p| path.starts_with(p))
}

/// Lenient boolean parse for the `PLACE_REQUIRE_AUTH` flag:
/// `1`/`true`/`yes`/`on` (case-insensitive, surrounding whitespace
/// ignored) ⇒ `true`; everything else (incl. empty, `0`, junk) ⇒
/// `false`.
#[must_use]
pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Read the blanket-enforcement flag from `PLACE_REQUIRE_AUTH`. Off by
/// default (unset/blank/junk ⇒ `false`). Read once at router
/// construction — changing the flag requires a restart.
#[must_use]
pub fn require_auth_from_env() -> bool {
    parse_bool(&std::env::var("PLACE_REQUIRE_AUTH").unwrap_or_default())
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

/// Load the ABAC policy: `PLACE_ABAC_POLICY` (inline JSON) wins, then
/// `PLACE_ABAC_POLICY_FILE` (path to a JSON file), else the built-in
/// default policy. A present-but-unparsable policy (bad JSON, unknown
/// effect/action names, unreadable file) `tracing::warn!`s and falls
/// back to the default — the service always boots, matching the
/// key-fetch posture. Read once at router construction; restart to
/// change.
#[must_use]
pub fn policy_from_env() -> Policy {
    let source = std::env::var("PLACE_ABAC_POLICY")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| {
            let path = std::env::var("PLACE_ABAC_POLICY_FILE")
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

/// The process-wide **hot-reloadable** token verifier.
///
/// Built lazily from `PLACE_PASETO_KEYS` on first use; [`init`] swaps in
/// the boot-time fetched key set and [`spawn_key_refresh`] swaps in a
/// re-fetched one periodically, so **a key rotation at the auth service
/// is picked up without restarting this service**. The blanket guard and
/// the `AuthUser` / `MaybeAuthUser` extractors all read this one holder,
/// so they can never disagree about which keys are current.
static VERIFIER: OnceLock<ReloadableVerifier> = OnceLock::new();

/// The process-wide reloadable verifier. Take a snapshot per request
/// with `verifier().current()`.
#[must_use]
pub fn verifier() -> &'static ReloadableVerifier {
    VERIFIER.get_or_init(|| ReloadableVerifier::new(super::state::verifier_from_env()))
}

/// Seed [`verifier`] before the app serves traffic (call once from
/// boot). With `PLACE_PASETO_KEYS_URL` set, the published key set is
/// fetched over HTTP once and swapped in; on failure — or with the URL
/// unset — the env-built verifier stands, so the service always boots.
pub async fn init() {
    if std::env::var("PLACE_PASETO_KEYS_URL")
        .ok()
        .is_some_and(|u| !u.trim().is_empty())
    {
        verifier().store(super::state::boot_verifier().await);
    }
}

/// Default key-set refresh interval (seconds) when
/// `PLACE_PASETO_KEYS_REFRESH_SECS` is unset. One hour: rotation is
/// infrequent, so a slow poll suffices.
const KEY_REFRESH_DEFAULT_SECS: u64 = 3600;

/// Spawn the background loop that re-fetches the published key set from
/// `PLACE_PASETO_KEYS_URL` and swaps it into the live [`verifier`].
///
/// A failed fetch **keeps the current key set** — a transient
/// auth-service outage must not lock every caller out. Interval from
/// `PLACE_PASETO_KEYS_REFRESH_SECS` (default
/// [`KEY_REFRESH_DEFAULT_SECS`]); `0` disables the loop. A no-op when
/// the URL is unset, since an env-supplied key set has nothing to
/// re-fetch.
pub fn spawn_key_refresh() {
    let Some(url) = std::env::var("PLACE_PASETO_KEYS_URL")
        .ok()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
    else {
        return;
    };
    let secs = std::env::var("PLACE_PASETO_KEYS_REFRESH_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(KEY_REFRESH_DEFAULT_SECS);
    if secs == 0 {
        return;
    }
    let issuer = super::state::env_or("PLACE_TOKEN_ISSUER", super::state::DEFAULT_ISSUER);
    let audience = super::state::env_or("PLACE_TOKEN_AUDIENCE", super::state::DEFAULT_AUDIENCE);
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(secs));
        ticker.tick().await; // the first tick is immediate — skip it
        loop {
            ticker.tick().await;
            match Verifier::from_paseto_keys_url(&url, &issuer, &audience).await {
                Ok(fetched) => {
                    tracing::info!(keys = fetched.key_count(), "refreshed PASETO key set");
                    verifier().store(fetched);
                }
                Err(error) => {
                    tracing::warn!(%error, "PASETO key-set refresh failed; keeping current keys");
                }
            }
        }
    });
    tracing::info!(secs, "polling PLACE_PASETO_KEYS_URL for key rotation");
}

/// The process-wide **hot-reloadable** ABAC policy, read by the blanket
/// guard per request. Swap it at runtime with [`reload_policy`].
#[must_use]
pub fn policy() -> &'static ReloadablePolicy {
    static POLICY: OnceLock<ReloadablePolicy> = OnceLock::new();
    POLICY.get_or_init(|| ReloadablePolicy::new(policy_from_env()))
}

/// Re-read the ABAC policy from the environment and swap it into the
/// live [`policy`] holder. A malformed policy falls back to the built-in
/// default, so a bad edit can never leave the service unprotected.
pub fn reload_policy() {
    policy().store(policy_from_env());
    tracing::info!("ABAC policy reloaded");
}

/// How often the policy-file watcher polls for a change (seconds).
const POLICY_WATCH_SECS: u64 = 15;

/// Spawn the background task that hot-reloads the ABAC policy when
/// `PLACE_ABAC_POLICY_FILE` changes on disk, so an operator can edit the
/// policy and have it take effect without a restart. Polls the file's
/// mtime (not an OS notify — this stays dependency-light).
///
/// A no-op when the variable is unset: an inline `PLACE_ABAC_POLICY` (or
/// the built-in default) has nothing to watch, and environment variables
/// do not change under a running process.
pub fn spawn_policy_watcher() {
    let Some(path) = std::env::var("PLACE_ABAC_POLICY_FILE")
        .ok()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
    else {
        return;
    };
    tokio::spawn(async move {
        let mut last = file_mtime(&path);
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(POLICY_WATCH_SECS));
        ticker.tick().await; // skip the immediate first tick
        loop {
            ticker.tick().await;
            let now = file_mtime(&path);
            if now != last {
                last = now;
                reload_policy();
            }
        }
    });
    tracing::info!(
        secs = POLICY_WATCH_SECS,
        "watching PLACE_ABAC_POLICY_FILE for changes"
    );
}

/// The last-modified time of `path`, or `None` when it cannot be read
/// (missing file, permission error, platform without mtime).
fn file_mtime(path: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

/// State the blanket-enforcement middleware closes over: just the
/// `PLACE_REQUIRE_AUTH` flag.
///
/// The verifier and policy are **not** snapshotted here — they live in
/// the process-wide reloadable holders ([`verifier`], [`policy`]) that
/// the middleware reads per request. Snapshotting them meant a key
/// rotation or a policy edit could reach the handlers but not the guard,
/// which is the half-applied state worst of all.
#[derive(Clone, Copy)]
pub struct EnforcementState {
    /// Whether blanket enforcement is on (read once at construction;
    /// restart to change).
    pub require_auth: bool,
}

impl EnforcementState {
    /// Read `PLACE_REQUIRE_AUTH`.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            require_auth: require_auth_from_env(),
        }
    }
}

impl Default for EnforcementState {
    /// Same as [`EnforcementState::from_env`].
    fn default() -> Self {
        Self::from_env()
    }
}

/// Axum middleware applying [`enforce`] to every request. Wire with
/// `axum::middleware::from_fn_with_state(enforcement, require_auth_middleware)`
/// on both router surfaces. A near-noop when the flag is off.
pub async fn require_auth_middleware(
    State(enforcement): State<EnforcementState>,
    request: Request,
    next: Next,
) -> Response {
    // Snapshots taken per request: whatever the refresh loop and the
    // policy watcher have most recently swapped in.
    let verifier = verifier().current();
    let policy = policy().current();
    let decision = enforce(
        enforcement.require_auth,
        request.method(),
        request.uri().path(),
        request.headers(),
        &verifier,
        &policy,
    );
    match decision {
        Ok(()) => next.run(request).await,
        Err(rejection) => rejection.into_response(),
    }
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

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // The live holder, not an `AppState` snapshot: a rotated key set
        // must reach the extractors and the guard together.
        bearer_claims(&parts.headers, &verifier().current()).map(AuthUser)
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
    /// rest (including empty, `0`, and junk).
    #[test]
    fn test_parse_bool_truthy_and_falsy() {
        for truthy in [
            "1", "true", "TRUE", "True", "yes", "YES", "on", " on ", "ON",
        ] {
            assert!(parse_bool(truthy), "{truthy:?} should parse true");
        }
        for falsy in ["", " ", "0", "false", "no", "off", "junk", "2", "enabled"] {
            assert!(!parse_bool(falsy), "{falsy:?} should parse false");
        }
    }

    /// The default policy the enforcement tests share (what an
    /// unconfigured service uses).
    fn policy() -> Policy {
        Policy::default_policy()
    }

    /// Enforcement off ⇒ a protected path passes with no token
    /// (today's default behaviour stays intact) — for reads and
    /// mutations alike (no authn and no authz when the flag is off).
    #[test]
    fn test_enforce_off_allows_protected_without_token() {
        let (verifier, policy) = (verifier(), policy());
        for method in [Method::GET, Method::POST, Method::DELETE] {
            assert!(
                enforce(
                    false,
                    &method,
                    "/api/places",
                    &HeaderMap::new(),
                    &verifier,
                    &policy
                )
                .is_ok(),
                "{method} should pass with enforcement off"
            );
        }
    }

    /// Enforcement on ⇒ every allow-listed public path (health probes,
    /// `OpenAPI` doc, Swagger UI, Prometheus metrics) still passes
    /// without a token.
    #[test]
    fn test_enforce_on_allows_public_paths() {
        let (verifier, policy) = (verifier(), policy());
        for path in [
            "/api/health",
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

    /// Enforcement on, protected path, no token ⇒ `401`. `/api/whoami`
    /// is deliberately not on the allow-list — it requires a token like
    /// every other `/api` route.
    #[test]
    fn test_enforce_on_protected_without_token_is_401() {
        let (verifier, policy) = (verifier(), policy());
        for path in ["/api/places", "/api/places/search", "/api/whoami"] {
            let err = enforce(
                true,
                &Method::GET,
                path,
                &HeaderMap::new(),
                &verifier,
                &policy,
            )
            .unwrap_err();
            assert_eq!(err.0, StatusCode::UNAUTHORIZED, "{path} should 401");
        }
    }

    /// Enforcement on, protected path, valid token ⇒ a read passes.
    #[test]
    fn test_enforce_on_protected_with_valid_token_is_ok() {
        let token = sign(10_000_000_000);
        assert!(
            enforce(
                true,
                &Method::GET,
                "/api/places",
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
            "/api/places",
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
            "/api/places",
            &bearer(&token),
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
            assert_eq!(derive_action(&method, "/api/places"), Action::Read);
        }
        assert_eq!(
            derive_action(&Method::DELETE, "/api/places/1"),
            Action::Delete
        );
        for path in [
            "/api/places/merge",
            "/api/places/deduplicate",
            "/api/places/import",
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
        assert_eq!(derive_action(&Method::POST, "/api/places"), Action::Write);
        assert_eq!(
            derive_action(&Method::POST, "/api/places/check-duplicates"),
            Action::Write
        );
        assert_eq!(derive_action(&Method::PUT, "/api/places/1"), Action::Write);
        assert_eq!(
            derive_action(&Method::PATCH, "/api/places/1"),
            Action::Write
        );
        // GET on a destructive-suffixed path is still a read — only
        // POST consults the suffix list.
        assert_eq!(
            derive_action(&Method::GET, "/api/places/merge"),
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
                "/api/places",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok()
        );
        let err = enforce(
            true,
            &Method::POST,
            "/api/places",
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
                    "/api/places",
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
            "/api/places/1",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(delete.0, StatusCode::FORBIDDEN);
        let merge = enforce(
            true,
            &Method::POST,
            "/api/places/merge",
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
                "/api/places/1",
                &bearer(&token),
                &verifier,
                &policy
            )
            .is_ok()
        );
        for path in ["/api/places/merge", "/api/places/deduplicate"] {
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
            (Method::GET, "/api/places"),
            (Method::POST, "/api/places"),
            (Method::PUT, "/api/places/1"),
            (Method::DELETE, "/api/places/1"),
            (Method::POST, "/api/places/merge"),
            (Method::POST, "/api/places/deduplicate"),
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
            "/api/places",
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
                "/api/places",
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
            "/api/places",
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
            "/api/places",
            &bearer(&token),
            &verifier,
            &policy,
        )
        .unwrap_err();
        assert_eq!(denied.0, StatusCode::FORBIDDEN);
        assert_eq!(denied.1, "default deny");
    }

    /// `policy_from_env` never breaks boot: bad inline JSON falls back
    /// to the built-in default policy. (Env mutation is process-global,
    /// so this test restores the prior state before returning.)
    #[test]
    fn test_policy_from_env_bad_json_falls_back_to_default() {
        // Serialize access to the env var against other tests.
        let parsed = Policy::from_json("{ not json").map(|_| ()).unwrap_err();
        let _ = parsed; // malformed JSON is an Err, never a panic
        // The pure fallback path `policy_from_env` takes on parse
        // failure is `Policy::default_policy()`:
        assert_eq!(
            Policy::from_json("{ not json").unwrap_or_else(|_| Policy::default_policy()),
            Policy::default_policy()
        );
    }

    /// Boot-time HTTP key fetch: a local ephemeral-port listener plays
    /// the auth service's `/.well-known/paseto-keys` role, serving the
    /// in-process test key set. The fetch-built verifier holds that key
    /// and accepts a token signed by it — no real auth service needed.
    #[tokio::test]
    async fn test_verifier_from_url_fetch_wins() {
        let keys = test_keys();
        let app = axum::Router::new().route(
            "/.well-known/paseto-keys",
            axum::routing::get(move || async move { Json(keys) }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve key set");
        });
        let url = format!("http://{addr}/.well-known/paseto-keys");
        let fetched = crate::api::rest::state::verifier_from_url_or_env(&url).await;
        assert_eq!(fetched.key_count(), 1);
        let token = sign(10_000_000_000);
        let claims =
            bearer_claims(&bearer(&token), &fetched).expect("fetched key set verifies token");
        assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
    }

    /// Boot-time HTTP key fetch failure: nothing listens on port 1, so
    /// the fetch fails fast and the builder falls back to the env path
    /// without panicking. `PLACE_PASETO_KEYS` is unset in the test
    /// environment, so the fallback is the empty reject-all key set —
    /// the service-always-boots guarantee.
    #[tokio::test]
    async fn test_verifier_from_url_failure_falls_back_to_env_path() {
        let fallback =
            crate::api::rest::state::verifier_from_url_or_env("http://127.0.0.1:1/").await;
        assert_eq!(fallback.key_count(), 0);
        assert!(fallback.verify(&sign(10_000_000_000)).is_err());
    }
}
