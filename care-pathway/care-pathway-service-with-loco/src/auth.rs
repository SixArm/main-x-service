//! Bearer-token authentication for the care-pathway API.
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
//! The process-wide [`verifier`] is built once from the environment:
//!
//! - `CARE_PATHWAY_PASETO_KEYS` — the Ed25519 key set (JSON, OKP/Ed25519
//!   JWK form) the auth service publishes at `/.well-known/paseto-keys`.
//!   Absent ⇒ an empty key set, so every token is rejected (the service
//!   still boots).
//! - `CARE_PATHWAY_TOKEN_ISSUER` — expected `iss` (default
//!   `authentication-service`).
//! - `CARE_PATHWAY_TOKEN_AUDIENCE` — expected `aud` (default
//!   `main-x-service`).
//!
//! Fetching the key set over HTTP from the auth service at boot (instead of
//! injecting it via env) is a follow-up — see spec §13.
//!
//! ## Blanket enforcement
//!
//! When `CARE_PATHWAY_REQUIRE_AUTH` is truthy (`1`/`true`/`yes`/`on`,
//! case-insensitive), the [`enforce`] decision — wired as an Axum
//! middleware layer in `src/app.rs` — requires a valid bearer token on
//! every route except the public health/ping, OpenAPI/Swagger, and
//! Prometheus metrics paths (see [`is_public_path`]). It is **off by
//! default**: unset/blank/junk
//! ⇒ today's behaviour, where the extractor is opt-in per handler and
//! `GET /api/care-pathways/whoami` proves end-to-end verification.
//! Activation is an operations decision once the SSO token flow is live;
//! see `agents/share/authentication-sessions.md` and
//! `agents/share/jwt-enforcement.md` for the family-wide contract.

use std::sync::{Arc, OnceLock};

use authentication_verifier::{Claims, Verifier};
use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};

/// Default issuer expected in tokens (`iss`).
const DEFAULT_ISSUER: &str = "authentication-service";
/// Default audience expected in tokens (`aud`).
const DEFAULT_AUDIENCE: &str = "main-x-service";

/// The process-wide token verifier, built from the environment on first
/// use (see the module docs). Shared behind an `Arc` and read-only.
#[must_use]
pub fn verifier() -> &'static Arc<Verifier> {
    static VERIFIER: OnceLock<Arc<Verifier>> = OnceLock::new();
    VERIFIER.get_or_init(|| Arc::new(build_from_env()))
}

/// Whether blanket `/api/*` enforcement is on, read once from
/// `CARE_PATHWAY_REQUIRE_AUTH` and cached. Off by default — see the
/// module docs and `agents/share/jwt-enforcement.md`. Mirrors
/// [`verifier`]: a process-wide `OnceLock` built from the environment.
#[must_use]
pub fn require_auth() -> bool {
    static REQUIRE_AUTH: OnceLock<bool> = OnceLock::new();
    *REQUIRE_AUTH
        .get_or_init(|| parse_bool(&std::env::var("CARE_PATHWAY_REQUIRE_AUTH").unwrap_or_default()))
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

/// The blanket-enforcement decision. `Ok(())` ⇒ let the request through;
/// `Err((401, msg))` ⇒ reject. Pure: the caller passes the flag, path,
/// headers and verifier, so it is fully unit-testable without booting the
/// app or a database.
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
    let issuer = env_or("CARE_PATHWAY_TOKEN_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("CARE_PATHWAY_TOKEN_AUDIENCE", DEFAULT_AUDIENCE);
    let keys = std::env::var("CARE_PATHWAY_PASETO_KEYS")
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
    use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
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

    /// Enforcement off ⇒ a protected path passes with no token.
    #[test]
    fn enforce_off_allows_without_token() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        assert!(enforce(false, "/api/care-pathways", &HeaderMap::new(), &verifier).is_ok());
    }

    /// Enforcement on ⇒ the public paths (health/ping, `OpenAPI`,
    /// Swagger UI, Prometheus metrics) still pass without a token.
    #[test]
    fn enforce_on_allows_public_paths() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        for path in [
            "/_health",
            "/_ping",
            "/api-docs/openapi.json",
            "/swagger-ui",
            "/swagger-ui/index.html",
            "/metrics.prom",
        ] {
            assert!(
                enforce(true, path, &HeaderMap::new(), &verifier).is_ok(),
                "{path} should be public"
            );
        }
    }

    /// Enforcement on, protected path, no token ⇒ `401`.
    #[test]
    fn enforce_on_protected_without_token_is_401() {
        let (keys, _) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let err = enforce(true, "/api/care-pathways", &HeaderMap::new(), &verifier).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on, protected path, valid token ⇒ passes.
    #[test]
    fn enforce_on_protected_with_valid_token_is_ok() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, 10_000_000_000);
        assert!(enforce(true, "/api/care-pathways", &bearer(&token), &verifier).is_ok());
    }

    /// Enforcement on, protected path, expired token ⇒ `401`.
    #[test]
    fn enforce_on_protected_with_expired_token_is_401() {
        let (keys, kid) = test_keys_and_kid();
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, -60);
        let err = enforce(true, "/api/care-pathways", &bearer(&token), &verifier).unwrap_err();
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
        let err = enforce(true, "/api/care-pathways", &bearer(&token), &verifier).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }
}
