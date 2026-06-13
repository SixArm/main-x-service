//! RS256 JWT issuance + verification + JWKS publication.
//!
//! This crate is the federation's single auth provider. Tokens are
//! signed with an RSA private key and verified by every other service
//! *offline* against the public key set published at
//! `/.well-known/jwks.json`. There is no shared secret and no
//! per-request introspection call — services fetch the JWKS once and
//! verify signatures locally.
//!
//! Key material is loaded from the environment (production) or from the
//! committed dev keypair under `config/keys/` (development). See
//! [`load_keys`] for the resolution order.

use std::sync::OnceLock;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use base64::Engine;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Default access-token lifetime (seconds). Deliberately short: with
/// offline JWKS verification, revoked tokens remain valid at peer
/// services until they expire, so we keep the window small.
const DEFAULT_EXPIRATION_SECS: i64 = 3600;

/// Resolved signing/verification material plus the published JWKS.
pub struct AuthKeys {
    encoding: EncodingKey,
    decoding: DecodingKey,
    /// JWK key id — SHA-256 thumbprint of the public modulus.
    pub kid: String,
    /// `iss` claim and JWKS issuer.
    pub issuer: String,
    /// `aud` claim — the federation audience.
    pub audience: String,
    /// Access-token lifetime in seconds.
    pub expiration: i64,
    /// Pre-rendered JWKS document served at `/.well-known/jwks.json`.
    pub jwks: serde_json::Value,
}

/// JWT claims. `sub` carries the user's public id (`pid`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user `pid` (UUID string).
    pub sub: String,
    /// User email, for convenience at the edge.
    pub email: String,
    /// Display name.
    pub name: String,
    /// Issuer.
    pub iss: String,
    /// Audience.
    pub aud: String,
    /// Expiry (unix seconds).
    pub exp: i64,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// JWT id — also the `sessions.jid`, enabling local revocation.
    pub jti: String,
}

/// Errors from key loading or token handling.
#[derive(Debug)]
pub enum AuthError {
    /// Key material could not be loaded or parsed.
    Keys(String),
    /// Token signing failed.
    Sign(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Keys(m) => write!(f, "jwt key error: {m}"),
            AuthError::Sign(m) => write!(f, "jwt sign error: {m}"),
        }
    }
}

impl std::error::Error for AuthError {}

static KEYS: OnceLock<AuthKeys> = OnceLock::new();

/// Process-wide accessor for the resolved key material. Initialised on
/// first use; a misconfiguration here is a fatal boot error, so it
/// panics with actionable context rather than degrading silently.
pub fn keys() -> &'static AuthKeys {
    KEYS.get_or_init(|| {
        load_keys().unwrap_or_else(|e| {
            panic!(
                "failed to load JWT keys ({e}). Set JWT_PRIVATE_KEY_FILE / \
                 JWT_PUBLIC_KEY_FILE (or JWT_PRIVATE_KEY_PEM / JWT_PUBLIC_KEY_PEM), \
                 or keep the dev keypair at config/keys/jwt_{{private,public}}_dev.pem"
            )
        })
    })
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

/// Resolve key material. PEM is taken inline from `JWT_PRIVATE_KEY_PEM`
/// / `JWT_PUBLIC_KEY_PEM` when set, else read from the files named by
/// `JWT_PRIVATE_KEY_FILE` / `JWT_PUBLIC_KEY_FILE`, which default to the
/// committed dev keypair.
///
/// # Errors
///
/// Returns [`AuthError::Keys`] when a key is missing or not valid RSA.
pub fn load_keys() -> Result<AuthKeys, AuthError> {
    let private_pem = match std::env::var("JWT_PRIVATE_KEY_PEM") {
        Ok(pem) => pem,
        Err(_) => {
            let path = env_or("JWT_PRIVATE_KEY_FILE", "config/keys/jwt_private_dev.pem");
            std::fs::read_to_string(&path)
                .map_err(|e| AuthError::Keys(format!("read {path}: {e}")))?
        }
    };
    let public_pem = match std::env::var("JWT_PUBLIC_KEY_PEM") {
        Ok(pem) => pem,
        Err(_) => {
            let path = env_or("JWT_PUBLIC_KEY_FILE", "config/keys/jwt_public_dev.pem");
            std::fs::read_to_string(&path)
                .map_err(|e| AuthError::Keys(format!("read {path}: {e}")))?
        }
    };

    let encoding = EncodingKey::from_rsa_pem(private_pem.as_bytes())
        .map_err(|e| AuthError::Keys(format!("private key: {e}")))?;
    let decoding = DecodingKey::from_rsa_pem(public_pem.as_bytes())
        .map_err(|e| AuthError::Keys(format!("public key: {e}")))?;

    // Derive JWK (n, e) + a stable kid from the public key.
    let pub_key = rsa::RsaPublicKey::from_public_key_pem(&public_pem)
        .map_err(|e| AuthError::Keys(format!("parse public key: {e}")))?;
    let n_bytes = pub_key.n().to_bytes_be();
    let e_bytes = pub_key.e().to_bytes_be();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let n = b64.encode(&n_bytes);
    let e = b64.encode(&e_bytes);
    let kid = b64.encode(Sha256::digest(&n_bytes));

    let issuer = env_or("JWT_ISSUER", "authentication-service");
    let audience = env_or("JWT_AUDIENCE", "main-x-service");
    let expiration = std::env::var("JWT_EXPIRATION")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_EXPIRATION_SECS);

    let jwks = serde_json::json!({
        "keys": [{
            "kty": "RSA",
            "use": "sig",
            "alg": "RS256",
            "kid": kid,
            "n": n,
            "e": e,
        }]
    });

    Ok(AuthKeys {
        encoding,
        decoding,
        kid,
        issuer,
        audience,
        expiration,
        jwks,
    })
}

/// Sign an RS256 access token for a user. Returns the token, its `jti`
/// (for the session row), and its expiry (unix seconds).
///
/// # Errors
///
/// Returns [`AuthError::Sign`] when encoding fails.
pub fn sign_access_token(
    user_pid: &str,
    email: &str,
    name: &str,
) -> Result<(String, String, i64), AuthError> {
    let k = keys();
    let now = chrono::Utc::now().timestamp();
    let exp = now + k.expiration;
    let jti = uuid::Uuid::new_v4().to_string();
    let claims = Claims {
        sub: user_pid.to_string(),
        email: email.to_string(),
        name: name.to_string(),
        iss: k.issuer.clone(),
        aud: k.audience.clone(),
        exp,
        iat: now,
        jti: jti.clone(),
    };
    let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some(k.kid.clone());
    let token =
        encode(&header, &claims, &k.encoding).map_err(|e| AuthError::Sign(e.to_string()))?;
    Ok((token, jti, exp))
}

/// Verify an RS256 token against the local public key, checking issuer,
/// audience, and expiry.
///
/// # Errors
///
/// Returns the underlying `jsonwebtoken` error on any validation failure.
pub fn verify_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let k = keys();
    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(std::slice::from_ref(&k.issuer));
    validation.set_audience(std::slice::from_ref(&k.audience));
    decode::<Claims>(token, &k.decoding, &validation).map(|data| data.claims)
}

/// Axum extractor for a verified bearer token. Pulls `Authorization:
/// Bearer <jwt>`, verifies the RS256 signature + claims, and yields the
/// [`Claims`]. Stateless — it does not touch the database, so peer
/// services can reuse the same verification logic. Revocation (signout)
/// is enforced separately by handlers that consult the `sessions` table.
pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing authorization header"))?;
        let token = header
            .strip_prefix("Bearer ")
            .or_else(|| header.strip_prefix("bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "expected bearer token"))?;
        let claims =
            verify_token(token.trim()).map_err(|_| (StatusCode::UNAUTHORIZED, "invalid token"))?;
        Ok(AuthUser(claims))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // These run against the committed dev keypair under config/keys/,
    // resolved relative to the crate root (cargo test's working dir).
    // No database required.

    #[test]
    fn jwks_publishes_one_rsa_signing_key() {
        let jwks = &keys().jwks;
        let key = &jwks["keys"][0];
        assert_eq!(key["kty"], "RSA");
        assert_eq!(key["use"], "sig");
        assert_eq!(key["alg"], "RS256");
        assert!(key["kid"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(key["n"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(key["e"].as_str().is_some_and(|s| !s.is_empty()));
        // The published kid must match the one stamped into token headers.
        assert_eq!(key["kid"].as_str().unwrap(), keys().kid);
    }

    #[test]
    fn sign_then_verify_round_trips_claims() {
        let pid = uuid::Uuid::new_v4().to_string();
        let (token, jti, exp) =
            sign_access_token(&pid, "alice@example.com", "Alice").expect("sign");
        assert!(!jti.is_empty());
        assert!(exp > chrono::Utc::now().timestamp());

        let claims = verify_token(&token).expect("verify");
        assert_eq!(claims.sub, pid);
        assert_eq!(claims.email, "alice@example.com");
        assert_eq!(claims.name, "Alice");
        assert_eq!(claims.iss, keys().issuer);
        assert_eq!(claims.aud, keys().audience);
        assert_eq!(claims.jti, jti);
    }

    #[test]
    fn tampered_token_is_rejected() {
        let (token, _, _) = sign_access_token("pid", "a@example.com", "A").expect("sign");
        // Flip the last character of the signature segment.
        let mut bytes = token.into_bytes();
        let last = bytes.len() - 1;
        bytes[last] ^= 0b0000_0001;
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(verify_token(&tampered).is_err());
    }

    #[test]
    fn garbage_token_is_rejected() {
        assert!(verify_token("not.a.jwt").is_err());
        assert!(verify_token("").is_err());
    }
}
