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
//! When `PERSON_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
//! case-insensitive), the [`enforce`] decision — wired as an Axum
//! middleware layer on both router surfaces (`create_router` and the
//! loco `after_routes` hook) — requires a valid bearer token on every
//! route except the public allow-list in [`PUBLIC_PATHS`]. It is **off
//! by default**: unset/blank/junk ⇒ today's behaviour, where the
//! extractor is opt-in per handler and `GET /api/whoami` proves
//! end-to-end verification. The flag is read **once at router
//! construction** ([`Enforcement::from_env`]), so changing the
//! environment variable requires a restart. Activation is an
//! operations decision once the SSO token flow is live; see
//! `agents/share/jwt-enforcement.md` for the family-wide contract.

use std::sync::Arc;

use authentication_verifier::{Claims, Verifier};
use axum::Json;
use axum::extract::{FromRef, FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::state::AppState;

/// Paths that stay public even when blanket enforcement is on. This is
/// what the crate actually serves without a token:
///
/// - `/api/health` — this service's own health endpoint (under `/api`);
/// - `/_health` / `/_ping` — loco's default health/ping routes (present
///   on the loco-mounted surface only);
/// - `/api-docs/openapi.json` — the `OpenAPI` 3 document;
/// - `/metrics.prom` — the root-mounted Prometheus scrape path (outside
///   `/api`, but the middleware layers the whole router, so it is
///   allow-listed explicitly for scrapers).
///
/// The Swagger UI (`/swagger-ui` + everything under `/swagger-ui/`) is
/// also public, matched by prefix in [`is_public_path`]. Everything
/// else requires a valid bearer token when enforcement is on.
pub const PUBLIC_PATHS: [&str; 5] = [
    "/api/health",
    "/_health",
    "/_ping",
    "/api-docs/openapi.json",
    "/metrics.prom",
];

/// Whether `path` is on the public allow-list ([`PUBLIC_PATHS`] plus
/// the Swagger UI prefix).
fn is_public_path(path: &str) -> bool {
    PUBLIC_PATHS.contains(&path) || path == "/swagger-ui" || path.starts_with("/swagger-ui/")
}

/// Lenient boolean parse for the `PERSON_REQUIRE_AUTH` flag:
/// `1`/`true`/`yes`/`on` (case-insensitive, surrounding whitespace
/// ignored) ⇒ `true`; everything else (incl. empty/unset/junk) ⇒
/// `false`. Family contract: `agents/share/jwt-enforcement.md`.
#[must_use]
pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Read the `PERSON_REQUIRE_AUTH` flag from the environment via
/// [`parse_bool`]. Called once at router construction (see
/// [`Enforcement::from_env`]) — changing the variable requires a
/// restart.
#[must_use]
pub fn require_auth_from_env() -> bool {
    parse_bool(&std::env::var("PERSON_REQUIRE_AUTH").unwrap_or_default())
}

/// The blanket-enforcement decision. `Ok(())` ⇒ let the request
/// through; `Err((401, msg))` ⇒ reject. Pure: the caller passes the
/// flag, path, headers and verifier, so it is fully unit-testable
/// without booting the app or a database.
///
/// # Errors
///
/// `401` when enforcement is on, the path is not public, and the
/// request carries no valid bearer token
/// (missing/malformed/expired/tampered).
pub fn enforce(
    require_auth: bool,
    path: &str,
    headers: &HeaderMap,
    verifier: &Verifier,
) -> Result<(), (StatusCode, String)> {
    if !require_auth || is_public_path(path) {
        return Ok(());
    }
    bearer_claims(headers, verifier).map(|_| ())
}

/// State for the blanket-enforcement middleware: the flag (read once
/// from the environment at construction) and the shared PASETO
/// verifier. Cheap to clone (a `bool` + an `Arc`).
#[derive(Clone)]
pub struct Enforcement {
    /// Whether blanket enforcement is on (`PERSON_REQUIRE_AUTH`,
    /// snapshotted at construction — restart to change).
    pub require_auth: bool,
    /// The PASETO `v4.public` verifier requests are checked against.
    pub verifier: Arc<Verifier>,
}

impl Enforcement {
    /// Snapshot `PERSON_REQUIRE_AUTH` and pair it with the given
    /// verifier (normally `AppState::verifier`).
    #[must_use]
    pub fn from_env(verifier: Arc<Verifier>) -> Self {
        Self {
            require_auth: require_auth_from_env(),
            verifier,
        }
    }
}

/// Axum middleware applying the blanket [`enforce`] decision to every
/// request on the router it is layered onto. A near-noop when the flag
/// is off, so it is wired unconditionally on both router surfaces and
/// `PERSON_REQUIRE_AUTH` is the only switch.
pub async fn require_auth_middleware(
    State(enforcement): State<Enforcement>,
    request: Request,
    next: Next,
) -> Response {
    match enforce(
        enforcement.require_auth,
        request.uri().path(),
        request.headers(),
        &enforcement.verifier,
    ) {
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

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = AppState::from_ref(state);
        bearer_claims(&parts.headers, &app.verifier).map(AuthUser)
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
    /// rest (including empty, `0`, and junk) — the
    /// `PERSON_REQUIRE_AUTH` flag semantics from
    /// `agents/share/jwt-enforcement.md`.
    #[test]
    fn test_parse_bool_truthy_and_falsy() {
        for truthy in ["1", "true", "TRUE", "Yes", "on", " on ", "ON"] {
            assert!(parse_bool(truthy), "{truthy:?} should parse true");
        }
        for falsy in ["", " ", "0", "false", "no", "off", "junk", "2"] {
            assert!(!parse_bool(falsy), "{falsy:?} should parse false");
        }
    }

    /// Enforcement off ⇒ a protected path passes with no token
    /// (default-off keeps today's behaviour).
    #[test]
    fn test_enforce_off_allows_without_token() {
        assert!(enforce(false, "/api/persons", &HeaderMap::new(), &verifier()).is_ok());
    }

    /// Enforcement on ⇒ every allow-listed public path (health/ping,
    /// `OpenAPI` doc, Swagger UI, Prometheus metrics) still passes
    /// without a token.
    #[test]
    fn test_enforce_on_allows_public_paths() {
        let verifier = verifier();
        for path in PUBLIC_PATHS {
            assert!(
                enforce(true, path, &HeaderMap::new(), &verifier).is_ok(),
                "{path} should be public"
            );
        }
        for path in ["/swagger-ui", "/swagger-ui/index.html"] {
            assert!(
                enforce(true, path, &HeaderMap::new(), &verifier).is_ok(),
                "{path} should be public"
            );
        }
    }

    /// Enforcement on, protected path, no token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_without_token_is_401() {
        let err = enforce(true, "/api/persons", &HeaderMap::new(), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, valid token ⇒ passes.
    #[test]
    fn test_enforce_on_protected_with_valid_token_is_ok() {
        let token = sign(10_000_000_000);
        assert!(enforce(true, "/api/persons", &bearer(&token), &verifier()).is_ok());
    }

    /// Enforcement on, protected path, expired token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_with_expired_token_is_401() {
        let token = sign(-60);
        let err = enforce(true, "/api/persons", &bearer(&token), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, tampered token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_with_tampered_token_is_401() {
        let mut token = sign(10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        let err = enforce(true, "/api/persons", &bearer(&token), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    // ---- Boot-time key-set fetch (`PERSON_PASETO_KEYS_URL`, T-1c) ----

    use crate::api::rest::state::verifier_from_url_or_env;

    /// Serve `test_keys()` from a local ephemeral-port HTTP listener,
    /// the way the auth service publishes `/.well-known/paseto-keys`.
    /// Returns the full key-set URL; the server task lives until the
    /// test process exits.
    async fn serve_test_keys() -> String {
        let router = axum::Router::new().route(
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

    /// URL unset ⇒ the env-key-set path: with no `PERSON_PASETO_KEYS`
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
}
