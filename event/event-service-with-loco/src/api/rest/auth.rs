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
//! When `EVENT_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
//! case-insensitive), the [`enforce`] decision — wired as the
//! [`require_auth_mw`] middleware on both router surfaces — requires a
//! valid bearer token on every route under [`API_PREFIX`] except the
//! public [`PUBLIC_API_PATHS`] allow-list. It is **off by default**:
//! unset/blank/junk ⇒ today's behaviour, where the extractor is opt-in
//! per handler and `GET /api/v1/whoami` proves end-to-end verification.
//! The flag is read once at [`AppState`] construction, so changing it
//! requires a restart. The `/fhir/*` `501` stubs sit outside the
//! `/api/v1` scope and stay public. Activation is an operations
//! decision once the SSO token flow is live; see
//! `agents/share/jwt-enforcement.md` for the family-wide contract.

use authentication_verifier::{Claims, Verifier};
use axum::Json;
use axum::extract::{FromRef, FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::state::AppState;

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

/// The API prefix under which blanket enforcement applies. Only paths
/// under this prefix are ever gated; everything else — loco's
/// `/_health` and `/_ping`, the `OpenAPI` doc `/api-docs/openapi.json`,
/// the Swagger UI at `/swagger-ui*`, the Prometheus scrape
/// `/metrics.prom`, and the `/fhir/*` `501 Not Implemented` stubs —
/// is outside the enforcement scope and always public.
pub const API_PREFIX: &str = "/api/v1";

/// API paths that stay public even when blanket enforcement is on: the
/// liveness probe, so orchestration needs no bearer token. This is the
/// complete allow-list inside [`API_PREFIX`]; every other `/api/v1/*`
/// route requires a valid bearer token when enforcement is on.
pub const PUBLIC_API_PATHS: &[&str] = &["/api/v1/health"];

/// Lenient boolean parse for the enforcement flag: `1`/`true`/`yes`/
/// `on` (case-insensitive, surrounding whitespace ignored) ⇒ `true`;
/// anything else (incl. empty / unset / `0` / junk) ⇒ `false`.
#[must_use]
pub fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Read the blanket-enforcement flag from `EVENT_REQUIRE_AUTH` via
/// [`parse_bool`] — **off by default**. Called once at [`AppState`]
/// construction and carried as `AppState::require_auth`, so changing
/// the variable requires a service restart.
#[must_use]
pub fn require_auth_from_env() -> bool {
    parse_bool(&std::env::var("EVENT_REQUIRE_AUTH").unwrap_or_default())
}

/// Whether `path` is under the enforced [`API_PREFIX`]. Segment-aware:
/// `/api/v1` and `/api/v1/...` match; nothing else does.
fn is_api_path(path: &str) -> bool {
    path.strip_prefix(API_PREFIX)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

/// The blanket-enforcement decision (family contract:
/// `agents/share/jwt-enforcement.md`). `Ok(())` ⇒ let the request
/// through; `Err((401, msg))` ⇒ reject. Pure: the caller passes the
/// flag, path, headers, and verifier, so it is fully unit-testable
/// without booting the app or a database.
///
/// # Errors
///
/// `401` when enforcement is on, the path is under [`API_PREFIX`] and
/// not in [`PUBLIC_API_PATHS`], and the request carries no valid bearer
/// token (missing / malformed / expired / tampered).
pub fn enforce(
    require_auth: bool,
    path: &str,
    headers: &HeaderMap,
    verifier: &Verifier,
) -> Result<(), (StatusCode, String)> {
    if !require_auth || !is_api_path(path) || PUBLIC_API_PATHS.contains(&path) {
        return Ok(());
    }
    bearer_claims(headers, verifier).map(|_| ())
}

/// Axum middleware wrapping [`enforce`]: reads the construction-time
/// flag (`AppState::require_auth`) and the shared verifier, and rejects
/// with `401` before any handler runs. Wired on **both** router
/// surfaces — the hand-written `create_router` and the loco router in
/// `App::after_routes` — via `axum::middleware::from_fn_with_state`.
/// Added unconditionally: it is a near-noop when the flag is off, so
/// the wiring stays static and the env var is the only switch.
pub async fn require_auth_mw(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let decision = enforce(
        state.require_auth,
        req.uri().path(),
        req.headers(),
        &state.verifier,
    );
    match decision {
        Ok(()) => next.run(req).await,
        Err(reject) => reject.into_response(),
    }
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

    /// `parse_bool` (the `EVENT_REQUIRE_AUTH` semantics) accepts the
    /// documented truthy set and rejects the rest, including empty,
    /// `0`, and junk.
    #[test]
    fn test_parse_bool_truthy_and_falsy() {
        for t in ["1", "true", "TRUE", "Yes", "on", " on ", "ON"] {
            assert!(parse_bool(t), "{t:?} should parse true");
        }
        for f in ["", " ", "0", "false", "no", "off", "junk", "2"] {
            assert!(!parse_bool(f), "{f:?} should parse false");
        }
    }

    /// Enforcement off ⇒ a protected `/api/v1` path passes with no
    /// token (today's default behaviour is unchanged).
    #[test]
    fn test_enforce_off_allows_without_token() {
        assert!(enforce(false, "/api/v1/events", &HeaderMap::new(), &verifier()).is_ok());
    }

    /// Enforcement on ⇒ the allow-listed `/api/v1/health` and every
    /// out-of-scope path (loco health/ping, `OpenAPI` doc, Swagger UI,
    /// Prometheus scrape, FHIR `501` stubs) still pass without a token.
    #[test]
    fn test_enforce_on_allows_public_and_out_of_scope_paths() {
        for path in [
            "/api/v1/health", // allow-listed inside /api/v1
            "/_health",       // loco default, outside /api/v1
            "/_ping",         // loco default, outside /api/v1
            "/api-docs/openapi.json",
            "/swagger-ui",
            "/swagger-ui/index.html",
            "/metrics.prom",
            "/fhir/Event", // 501 stub surface, outside /api/v1
            "/fhir/Event/00000000-0000-0000-0000-000000000000",
        ] {
            assert!(
                enforce(true, path, &HeaderMap::new(), &verifier()).is_ok(),
                "{path} should be public"
            );
        }
    }

    /// Enforcement on, protected path, no token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_without_token_is_401() {
        let err = enforce(true, "/api/v1/events", &HeaderMap::new(), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, valid token ⇒ passes.
    #[test]
    fn test_enforce_on_protected_with_valid_token_is_ok() {
        let token = sign(10_000_000_000);
        assert!(enforce(true, "/api/v1/events", &bearer(&token), &verifier()).is_ok());
    }

    /// Enforcement on, protected path, expired token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_with_expired_token_is_401() {
        let token = sign(-60);
        let err = enforce(true, "/api/v1/events", &bearer(&token), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, tampered token ⇒ `401`.
    #[test]
    fn test_enforce_on_protected_with_tampered_token_is_401() {
        let mut token = sign(10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        let err = enforce(true, "/api/v1/events", &bearer(&token), &verifier()).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }
}
