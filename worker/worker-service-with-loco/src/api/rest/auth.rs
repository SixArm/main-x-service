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

use std::sync::Arc;

use authentication_verifier::{Claims, Verifier};
use axum::extract::{FromRef, FromRequestParts, Request};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::{Json, Router};

use super::state::AppState;

/// Exact-match paths that stay public even when blanket enforcement is
/// on: loco's default health probes (`/_health`, `/_ping`), this crate's
/// own health endpoint (`/api/v1/health` — orchestration probes carry no
/// token), the served `OpenAPI` document, and the Prometheus scrape
/// endpoint (`/metrics.prom` — scrapers carry no token). Everything else
/// — the whole `/api/v1` surface and the `/fhir` surface (worker PII) —
/// requires a valid bearer token when enforcement is on.
pub const PUBLIC_PATHS: [&str; 5] = [
    "/_health",
    "/_ping",
    "/api/v1/health",
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

/// The blanket-enforcement decision. `Ok(())` ⇒ let the request through;
/// `Err((401, msg))` ⇒ reject. Pure: the caller passes the flag, path,
/// headers and verifier, so it is fully unit-testable without booting
/// the app or a database.
///
/// # Errors
///
/// `401` when enforcement is on, the path is not public, and the request
/// carries no valid bearer token (missing/malformed/expired/tampered).
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

/// Layer the blanket-enforcement middleware onto a finished router. The
/// flag and verifier are captured **at construction** (restart to
/// change); when the flag is off the middleware is a near-noop, so both
/// router surfaces wire it unconditionally and `WORKER_REQUIRE_AUTH` is
/// the only switch. Applied beneath the CORS layer so preflight
/// `OPTIONS` requests are answered by CORS before enforcement runs.
pub fn apply_enforcement(router: Router, require_auth: bool, verifier: Arc<Verifier>) -> Router {
    router.layer(axum::middleware::from_fn(
        move |req: Request, next: Next| {
            let verifier = Arc::clone(&verifier);
            async move {
                match enforce(require_auth, req.uri().path(), req.headers(), &verifier) {
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

/// `GET /api/v1/whoami` — echo the verified claims of the bearer token.
/// Returns `401` when the token is missing, malformed, or fails
/// verification. Useful for confirming peer PASETO verification end to
/// end.
#[utoipa::path(
    get,
    path = "/api/v1/whoami",
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

    /// Enforcement off ⇒ a protected path passes with no token (today's
    /// default behaviour is preserved).
    #[test]
    fn test_enforce_off_allows_protected_without_token() {
        assert!(enforce(false, "/api/v1/workers", &HeaderMap::new(), &verifier()).is_ok());
    }

    /// Enforcement on ⇒ every allow-listed public path still passes
    /// without a token.
    #[test]
    fn test_enforce_on_allows_public_paths() {
        for path in PUBLIC_PATHS {
            assert!(
                enforce(true, path, &HeaderMap::new(), &verifier()).is_ok(),
                "{path} should be public"
            );
        }
        for path in ["/swagger-ui", "/swagger-ui/index.html"] {
            assert!(
                enforce(true, path, &HeaderMap::new(), &verifier()).is_ok(),
                "{path} should be public"
            );
        }
    }

    /// Enforcement on, protected path, no token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_without_token_is_401() {
        let err = enforce(true, "/api/v1/workers", &HeaderMap::new(), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, valid token ⇒ passes.
    #[test]
    fn test_enforce_on_protected_with_valid_token_is_ok() {
        let token = sign(10_000_000_000);
        assert!(enforce(true, "/api/v1/workers", &bearer(&token), &verifier()).is_ok());
    }

    /// Enforcement on, protected path, expired token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_with_expired_token_is_401() {
        let token = sign(-60);
        let err = enforce(true, "/api/v1/workers", &bearer(&token), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, tampered token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_with_tampered_token_is_401() {
        let mut token = sign(10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        let err = enforce(true, "/api/v1/workers", &bearer(&token), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// The FHIR surface is protected too — it serves worker PII, so it
    /// is deliberately not on the allow-list.
    #[test]
    fn test_enforce_on_fhir_without_token_is_401() {
        let err = enforce(true, "/fhir/Worker", &HeaderMap::new(), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
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
}
