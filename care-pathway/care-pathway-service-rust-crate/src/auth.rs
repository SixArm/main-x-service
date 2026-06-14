//! Bearer-token authentication for the care-pathway API.
//!
//! [`AuthUser`] is an Axum extractor that pulls `Authorization: Bearer
//! <jwt>`, verifies the RS256 signature, issuer, audience and expiry
//! against the [authentication-service](../../../authentication/authentication-service-rust-crate)
//! JWKS, and yields the verified [`Claims`]. Verification is stateless
//! and offline — no database hit, no introspection call — so any handler
//! can require authentication by taking an `AuthUser` argument, and a
//! handler that wants the caller identity *when present* (e.g. to stamp
//! an audit `actor`) takes [`MaybeAuthUser`] instead.
//!
//! ## JWKS source
//!
//! The process-wide [`verifier`] is built once from the environment:
//!
//! - `CARE_PATHWAY_JWKS` — the JWKS document (JSON) the auth service
//!   publishes at `/.well-known/jwks.json`. Absent ⇒ an empty key set,
//!   so every token is rejected (the service still boots).
//! - `CARE_PATHWAY_JWT_ISSUER` — expected `iss` (default
//!   `authentication-service`).
//! - `CARE_PATHWAY_JWT_AUDIENCE` — expected `aud` (default
//!   `main-x-service`).
//!
//! Fetching the JWKS over HTTP from the auth service at boot (instead of
//! injecting it via env) is a follow-up — see spec §13 T-7.
//!
//! ## Blanket enforcement
//!
//! Mandatory bearer-token auth on every `/api/*` route is wired via the
//! [`enforce`] decision function and an `axum::middleware::from_fn` layer
//! in `app.rs`. It is **off by default** and gated by the
//! `CARE_PATHWAY_REQUIRE_AUTH` env var ([`require_auth`]): `1`/`true`/
//! `yes`/`on` (case-insensitive) ⇒ enabled; anything else ⇒ disabled.
//! When off, behaviour is exactly as before (the extractor is available
//! for opt-in protection and actor capture, and
//! `GET /api/care-pathways/whoami` proves end-to-end verification).
//! Activation is an operations decision once the SSO token flow is live;
//! see the family contract in `agents/share/jwt-enforcement.md` and spec
//! §13 T-7.

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

/// Read env var `name`, falling back to `default` when it is unset or
/// blank (whitespace-only counts as blank). Used for the issuer/audience.
fn env_or(name: &str, default: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

/// Build the [`Verifier`] from the environment (issuer, audience, JWKS).
///
/// A missing or unparseable `CARE_PATHWAY_JWKS` degrades to an empty key
/// set (every token rejected) rather than failing to boot, and a JWKS
/// that fails to load falls back to [`empty_verifier`] for the same reason.
fn build_from_env() -> Verifier {
    let issuer = env_or("CARE_PATHWAY_JWT_ISSUER", DEFAULT_ISSUER);
    let audience = env_or("CARE_PATHWAY_JWT_AUDIENCE", DEFAULT_AUDIENCE);
    let jwks = std::env::var("CARE_PATHWAY_JWKS")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .unwrap_or_else(|| serde_json::json!({ "keys": [] }));
    Verifier::from_jwks_value(&jwks, &issuer, &audience)
        .unwrap_or_else(|_| empty_verifier(&issuer, &audience))
}

/// A verifier with no keys: rejects every token until a real JWKS is
/// configured. Infallible — an empty `keys` array always parses.
fn empty_verifier(issuer: &str, audience: &str) -> Verifier {
    let empty = serde_json::json!({ "keys": [] });
    Verifier::from_jwks_value(&empty, issuer, audience).expect("empty jwks always builds")
}

/// Parse a boolean-ish env value leniently: `1`/`true`/`yes`/`on`
/// (case-insensitive, trimmed) ⇒ `true`; anything else ⇒ `false`.
#[must_use]
pub fn parse_bool(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Whether blanket `/api/*` JWT enforcement is on, read once from
/// `CARE_PATHWAY_REQUIRE_AUTH` (see [`parse_bool`]). Off by default —
/// unset/blank ⇒ `false`. Mirrors [`verifier`] with a `OnceLock`.
#[must_use]
pub fn require_auth() -> bool {
    static REQUIRE_AUTH: OnceLock<bool> = OnceLock::new();
    *REQUIRE_AUTH
        .get_or_init(|| std::env::var("CARE_PATHWAY_REQUIRE_AUTH").is_ok_and(|v| parse_bool(&v)))
}

/// Paths that stay public even when enforcement is on: health/ping and
/// the `OpenAPI` doc + Swagger UI. Everything else under `/api` requires a
/// valid bearer token.
fn is_public_path(path: &str) -> bool {
    path == "/_health"
        || path == "/_ping"
        || path == "/api-docs/openapi.json"
        || path.starts_with("/swagger-ui")
}

/// The blanket-enforcement decision. `Ok(())` ⇒ let the request through;
/// `Err((401, msg))` ⇒ reject. Pure: caller passes the flag, path,
/// headers and verifier, so it is fully unit-testable.
///
/// # Errors
///
/// `401` when enforcement is on, the path is not public, and the request
/// carries no valid bearer token (see [`bearer_claims`]).
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

/// Extract and verify the bearer token from request headers. Pure (the
/// verifier is passed in), so it is unit-testable without the global.
///
/// # Errors
///
/// `401` when the `Authorization` header is missing, is not a bearer
/// token, or the token fails RS256 / issuer / audience / expiry
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

/// A request whose bearer token passed RS256 / issuer / audience /
/// expiry verification. The wrapped [`Claims`] identify the caller
/// (`sub` is the user `pid`). Taking this argument makes a handler
/// require authentication.
pub struct AuthUser(pub Claims);

impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    /// `401`-style rejection (status + message) when verification fails.
    type Rejection = (StatusCode, String);

    /// Verify the request's bearer token against the process-wide
    /// [`verifier`], yielding the [`AuthUser`] or a `401` rejection.
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

impl<S: Send + Sync> FromRequestParts<S> for MaybeAuthUser {
    /// Never rejects — extraction always succeeds (claims may be `None`).
    type Rejection = std::convert::Infallible;

    /// Try to verify the bearer token; on any failure yield
    /// `MaybeAuthUser(None)` rather than rejecting the request.
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Self(bearer_claims(&parts.headers, verifier()).ok()))
    }
}

/// Unit tests for the pure verification path. These mint RS256 tokens and
/// a matching JWKS in-process from a throwaway key, so they exercise the
/// real `Verifier` without a network/JWKS fetch and run DB-free on the
/// default `cargo test`.
#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use jsonwebtoken::{encode, EncodingKey, Header};
    use rsa::pkcs8::DecodePrivateKey;
    use rsa::traits::PublicKeyParts;

    /// Issuer used by the test tokens and verifier (matches the default).
    const ISSUER: &str = "authentication-service";
    /// Audience used by the test tokens and verifier (matches the default).
    const AUDIENCE: &str = "main-x-service";

    // A throwaway 2048-bit RSA key, used only to mint test tokens and a
    // matching JWKS in-process. Not a secret — never used in production.
    const TEST_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDfpGbZFF+Y1/MC
okA1EudchK0DgS4Sq9KV97TIJTJDzm2jwIhLiPYIcw9jkJMhtTa5GO23fRjq3D/S
s2k8DcA/9bZh/PVvQDYrRmDFL1xbnWTuH/YBLX+wPm+AXoikWAXn2+Hj0g+CsM0m
AYVhF/H3n6+A1J6yt7mqZbyhkCqsIzm6fy1kIO0hHkA3Su49z6lHbE/eob0/cuWr
vkHnkC+tPbgScaYYXwhe5kreZzzKpyBM/AHNIwCuruH+UoczQiL+nBjKt8adnAQM
x8wiiwH85VLBMVqxQM7jYfManszDoK9v9y9KhAYaWdbAvuaoGcild8MbMGBCXfEi
1er5s9rJAgMBAAECggEACJY9uLIL5Z4iX4SD+0W0NRzcWRkZqsUxexPyu13qQH4+
6XMCs1vD4+5/Cxn0UZlX9jgbJdjGHyZrxkGglcS1ZVSgNxeeWNzZ95lmgnZfZnPJ
pKGnhsNiIWM+/BUlIUzxP8Yf4JPNMqhqiBNg3/8qXuahDMzydvbk8xegQYJ75ktg
PXIE4VlIsHNAMBwRnMhLnvfhlTHgGb/Cs1zQYQFP2ocYxCxclv6ddNKoYYmsmWyj
5bCVMbNzo52wXnw0e5IH1Sr6kL9ptg8OOwR++y0F2jqyGQQlZ6m5oyxfGmZnQ4v4
aSPeN/Up5OzKbezXI5OC89uFYuMnhxN/6SuKbN0X8QKBgQD0dt7DTt6nz36vExOy
6e6pg4sO358u29ZYcMWZIPgZmjJwrBuzkIyDhebP4Bd93+1Xno/AqEHr/BOgiagp
kwwRBkkThUjHgwes8ZZ+Kk9zTH3JccQeIu9n5H6sq+cpfZQjyeVcRmKrOm2i7jKy
3X2viPopO7nXq3AulA9bj91tjQKBgQDqMgADMPK1usGD+x9aKykFRBVJS/odtF6d
77Pg6wv6JbwH9Q6wXcjo8c9OEwam5bdgmz9vVkHDPBXZKMfFP8gfAX/pupEqs0Yk
6MsplPBFNeSr5Uq9RUI/Qun2JxP61sK5UQ7wb3TCVJx5RM2LNSDXwnl5EpS07DbO
qfX546E9LQKBgQDPoAnQXTLz2WHh9dTcNpyxsfwv1LNQy/t/P8BDLuIodHL0iOg6
GMGOjvIaiVvKV54vtYan/P+IGp7c0S1WqgIsj2cPQjsu39VsB/9mBi9WYJfQuGP1
qHwmg8UmiBWbgoGH59h6B3mTrdsh7yZ2DXHK3Q6CaKyNRJjRpoRoooZQnQKBgHMF
CYKHuLxOM5qZbCWBywy+CmJMQVPzcQDKaCLP7br2a2nRDlzKQtE9aZ4jtAGmErEM
rlQFHhk/2k8kOzECCUxJFUR6j69UCuA3wQf0ESk8tclCvLlGWanuOC/fs21fqpUP
XXHym3qRyaO5ieWTu0ScS8KNwKE23hgT2y3WgSslAoGAOqgLmjAkCu3crbc7aPIi
32rEFszCo1YY9iYtQizUO5yO01zPF/lfT2yNtu8KuotycH/+N8veT3hFYssc7IOT
Eq9K7IobXuoSOu4eR1SZoZ29lTRAqMCjFfdFdFgdhvN2nx8XXrYymNsKmT7rOLk3
cg5Tq5R846wbNyxrso8C988=
-----END PRIVATE KEY-----";

    /// The URL-safe, unpadded base64 engine JWK/JWS fields are encoded in.
    fn b64() -> base64::engine::general_purpose::GeneralPurpose {
        base64::engine::general_purpose::URL_SAFE_NO_PAD
    }

    /// Build a JWKS + matching `kid` from the test public key, the same
    /// way the auth service publishes it.
    fn test_jwks_and_kid() -> (serde_json::Value, String) {
        use sha2::{Digest, Sha256};
        let private = rsa::RsaPrivateKey::from_pkcs8_pem(TEST_PRIVATE_PEM).expect("parse pem");
        let public = private.to_public_key();
        let n = public.n().to_bytes_be();
        let e = public.e().to_bytes_be();
        let kid = b64().encode(Sha256::digest(&n));
        let jwks = serde_json::json!({
            "keys": [{
                "kty": "RSA", "use": "sig", "alg": "RS256",
                "kid": kid, "n": b64().encode(&n), "e": b64().encode(&e),
            }]
        });
        (jwks, kid)
    }

    /// Mint a signed RS256 token for the test key with header `kid` and an
    /// `exp` offset (seconds) relative to a fixed `iat`. A negative offset
    /// yields an already-expired token.
    fn sign(kid: &str, exp_offset_secs: i64) -> String {
        // A fixed `iat` keeps the token deterministic; `exp` is relative.
        let iat: i64 = 1_700_000_000;
        let claims = Claims {
            sub: "11111111-1111-1111-1111-111111111111".into(),
            email: "alice@example.com".into(),
            name: "Alice".into(),
            iss: ISSUER.into(),
            aud: AUDIENCE.into(),
            exp: iat + exp_offset_secs,
            iat,
            jti: "test-jti".into(),
        };
        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some(kid.to_string());
        let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_PEM.as_bytes()).expect("encoding key");
        encode(&header, &claims, &key).expect("sign")
    }

    /// Wrap a token in an `Authorization: Bearer <token>` header map.
    fn bearer(token: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        h
    }

    /// A valid token verifies and yields its claims (`sub`, `email`, …).
    #[test]
    fn valid_token_yields_claims() {
        let (jwks, kid) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        // A token that expires far in the future relative to its `iat`.
        let token = sign(&kid, 10_000_000_000);
        let claims = bearer_claims(&bearer(&token), &verifier).expect("valid token verifies");
        assert_eq!(claims.sub, "11111111-1111-1111-1111-111111111111");
        assert_eq!(claims.email, "alice@example.com");
    }

    /// No `Authorization` header ⇒ `401`.
    #[test]
    fn missing_header_is_401() {
        let (jwks, _) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        let err = bearer_claims(&HeaderMap::new(), &verifier).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// A non-bearer scheme (here `Basic`) ⇒ `401`.
    #[test]
    fn non_bearer_header_is_401() {
        let (jwks, _) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, "Basic abc123".parse().unwrap());
        assert_eq!(
            bearer_claims(&h, &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// An expired token (`exp` in the past) ⇒ `401`.
    #[test]
    fn expired_token_is_401() {
        let (jwks, kid) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        // exp = iat - 60: already expired.
        let token = sign(&kid, -60);
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// A signature-tampered token fails RS256 verification ⇒ `401`.
    #[test]
    fn tampered_token_is_401() {
        let (jwks, kid) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        let mut token = sign(&kid, 10_000_000_000);
        // Flip the last signature character.
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// The keyless [`empty_verifier`] rejects even an otherwise-valid
    /// token (no key to verify against) ⇒ `401`.
    #[test]
    fn empty_verifier_rejects_everything() {
        let verifier = empty_verifier(ISSUER, AUDIENCE);
        let (_, kid) = test_jwks_and_kid();
        let token = sign(&kid, 10_000_000_000);
        assert_eq!(
            bearer_claims(&bearer(&token), &verifier).unwrap_err().0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// [`parse_bool`] accepts the documented truthy spellings and rejects
    /// everything else (the `CARE_PATHWAY_REQUIRE_AUTH` grammar).
    #[test]
    fn parse_bool_truthy_and_falsy() {
        for t in ["1", "true", "TRUE", "yes", "Yes", "on", "ON", " true "] {
            assert!(parse_bool(t), "{t:?} should parse true");
        }
        for f in ["", "0", "false", "no", "off", "junk", "  "] {
            assert!(!parse_bool(f), "{f:?} should parse false");
        }
    }

    /// Enforcement off ⇒ [`enforce`] lets a protected path through even
    /// with no token (the default, fully back-compatible posture).
    #[test]
    fn enforce_off_allows_without_token() {
        let (jwks, _) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        // Enforcement off: even a protected path with no token is allowed.
        assert!(enforce(false, "/api/care-pathways", &HeaderMap::new(), &verifier).is_ok());
    }

    /// Enforcement on ⇒ the allow-listed public paths (health/ping, the
    /// `OpenAPI` doc, Swagger UI) still pass without a token.
    #[test]
    fn enforce_on_allows_public_paths() {
        let (jwks, _) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        let h = HeaderMap::new();
        for p in [
            "/_health",
            "/_ping",
            "/api-docs/openapi.json",
            "/swagger-ui",
            "/swagger-ui/index.html",
        ] {
            assert!(
                enforce(true, p, &h, &verifier).is_ok(),
                "{p} should be public"
            );
        }
    }

    /// Enforcement on + protected path + no token ⇒ `401`.
    #[test]
    fn enforce_on_protected_without_token_is_401() {
        let (jwks, _) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        let err = enforce(true, "/api/care-pathways", &HeaderMap::new(), &verifier).unwrap_err();
        assert_eq!(err.0, StatusCode::UNAUTHORIZED);
    }

    /// Enforcement on + protected path + valid token ⇒ allowed.
    #[test]
    fn enforce_on_protected_with_valid_token_is_ok() {
        let (jwks, kid) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, 10_000_000_000);
        assert!(enforce(true, "/api/care-pathways", &bearer(&token), &verifier).is_ok());
    }

    /// Enforcement on + protected path + expired token ⇒ `401`.
    #[test]
    fn enforce_on_protected_with_expired_token_is_401() {
        let (jwks, kid) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        let token = sign(&kid, -60);
        assert_eq!(
            enforce(true, "/api/care-pathways", &bearer(&token), &verifier)
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
    }

    /// Enforcement on + protected path + signature-tampered token ⇒ `401`.
    #[test]
    fn enforce_on_protected_with_tampered_token_is_401() {
        let (jwks, kid) = test_jwks_and_kid();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).unwrap();
        let mut token = sign(&kid, 10_000_000_000);
        let last = token.pop().unwrap();
        token.push(if last == 'a' { 'b' } else { 'a' });
        assert_eq!(
            enforce(true, "/api/care-pathways", &bearer(&token), &verifier)
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
    }
}
