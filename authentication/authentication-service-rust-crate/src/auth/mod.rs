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
//!
//! ## Key rotation (zero-downtime)
//!
//! [`AuthKeys`] is a **key set**, not a single key: one *primary*
//! signing key plus zero or more *additional* verify-only public keys.
//! [`sign_access_token`] always signs with the primary and stamps the
//! primary's `kid`; [`verify_token`] selects the verifying key by the
//! token header's `kid`, so a token signed by a key that has since been
//! rotated down to "additional" still verifies locally until it expires.
//! The JWKS publishes the whole set (primary first), so peers trust
//! every live `kid`. Additional keys load from
//! `JWT_ADDITIONAL_PUBLIC_KEY_FILES` / `JWT_ADDITIONAL_PUBLIC_KEY_PEMS`;
//! unset ⇒ just the primary (fully backward-compatible). See the spec
//! §8.4 key-management runbook for the operator rotation procedure.

use std::collections::HashMap;
use std::sync::OnceLock;

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use base64::Engine;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode};
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Default access-token lifetime (seconds). Deliberately short: with
/// offline JWKS verification, revoked tokens remain valid at peer
/// services until they expire, so we keep the window small.
const DEFAULT_EXPIRATION_SECS: i64 = 3600;

/// One RSA public verification key plus the `kid` it is published under.
/// Used both for the primary's verify side and for each verify-only
/// additional key.
struct VerifyKey {
    /// JWK key id — `base64url(SHA-256(big-endian modulus))`.
    kid: String,
    /// The decoding key used to check RS256 signatures.
    decoding: DecodingKey,
    /// Pre-rendered JWK (`kty`/`use`/`alg`/`kid`/`n`/`e`) for the JWKS.
    jwk: serde_json::Value,
}

/// Resolved signing/verification material plus the published JWKS.
///
/// Holds a **set** of keys: exactly one primary signing key (used by
/// [`sign_access_token`]) and zero or more additional verify-only public
/// keys (recently rotated-out keys whose still-live tokens must keep
/// verifying). [`verify_token`] selects among them by the token header
/// `kid`. The JWKS publishes all of them, primary first.
pub struct AuthKeys {
    /// Primary signing key.
    encoding: EncodingKey,
    /// Verification keys keyed by `kid`: the primary plus any additional.
    by_kid: HashMap<String, DecodingKey>,
    /// JWK key id of the **primary** key — stamped into every token
    /// header and the first entry of the JWKS. SHA-256 thumbprint of the
    /// primary public modulus.
    pub kid: String,
    /// `iss` claim and JWKS issuer.
    pub issuer: String,
    /// `aud` claim — the federation audience.
    pub audience: String,
    /// Access-token lifetime in seconds.
    pub expiration: i64,
    /// Pre-rendered JWKS document served at `/.well-known/jwks.json`.
    /// Carries every key in the set (primary first).
    pub jwks: serde_json::Value,
}

impl AuthKeys {
    /// Number of verification keys in the set (1 primary + additional).
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.by_kid.len()
    }
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
///
/// # Panics
///
/// Panics if key material cannot be loaded (see [`load_keys`]): missing
/// or invalid RSA PEM from the env/file sources.
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

/// Derive the verify-side material (`kid`, decoding key, JWK) from a
/// public-key PEM. The `kid` is `base64url(SHA-256(big-endian modulus))`,
/// stable across restarts and identical in token headers and the JWKS.
///
/// # Errors
///
/// Returns [`AuthError::Keys`] when the PEM is not a valid RSA public key.
fn verify_key_from_public_pem(public_pem: &str) -> Result<VerifyKey, AuthError> {
    let decoding = DecodingKey::from_rsa_pem(public_pem.as_bytes())
        .map_err(|e| AuthError::Keys(format!("public key: {e}")))?;
    let pub_key = rsa::RsaPublicKey::from_public_key_pem(public_pem)
        .map_err(|e| AuthError::Keys(format!("parse public key: {e}")))?;
    let n_bytes = pub_key.n().to_bytes_be();
    let e_bytes = pub_key.e().to_bytes_be();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let kid = b64.encode(Sha256::digest(&n_bytes));
    let jwk = serde_json::json!({
        "kty": "RSA",
        "use": "sig",
        "alg": "RS256",
        "kid": kid,
        "n": b64.encode(&n_bytes),
        "e": b64.encode(&e_bytes),
    });
    Ok(VerifyKey { kid, decoding, jwk })
}

/// Split an inline-PEM env value (`JWT_ADDITIONAL_PUBLIC_KEY_PEMS`) into
/// individual PEM blocks. Multiple keys may be separated by commas or
/// blank lines; we just split on the `-----END ...-----` boundary so the
/// caller can paste a concatenation of standard PEM blocks.
fn split_pems(blob: &str) -> Vec<String> {
    let mut pems = Vec::new();
    let mut current = String::new();
    for line in blob.lines() {
        let trimmed = line.trim();
        // A lone comma separator between blocks is ignored.
        if trimmed.is_empty() || trimmed == "," {
            continue;
        }
        current.push_str(trimmed);
        current.push('\n');
        if trimmed.starts_with("-----END") {
            pems.push(std::mem::take(&mut current));
        }
    }
    if !current.trim().is_empty() {
        pems.push(current);
    }
    pems
}

/// Resolve the *additional* verify-only public keys from the environment.
/// `JWT_ADDITIONAL_PUBLIC_KEY_FILES` is a comma-separated list of file
/// paths; `JWT_ADDITIONAL_PUBLIC_KEY_PEMS` carries inline PEM blocks
/// (comma- or newline-separated). Both are optional and may be combined;
/// unset/empty ⇒ no additional keys (backward-compatible single-key set).
///
/// # Errors
///
/// Returns [`AuthError::Keys`] when a listed file cannot be read or a PEM
/// is not a valid RSA public key.
fn additional_public_pems() -> Result<Vec<String>, AuthError> {
    let mut pems = Vec::new();
    if let Ok(files) = std::env::var("JWT_ADDITIONAL_PUBLIC_KEY_FILES") {
        for path in files.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let pem = std::fs::read_to_string(path)
                .map_err(|e| AuthError::Keys(format!("read additional key {path}: {e}")))?;
            pems.push(pem);
        }
    }
    if let Ok(inline) = std::env::var("JWT_ADDITIONAL_PUBLIC_KEY_PEMS") {
        pems.extend(split_pems(&inline));
    }
    Ok(pems)
}

/// Resolve key material. The primary signing key's PEM is taken inline
/// from `JWT_PRIVATE_KEY_PEM` / `JWT_PUBLIC_KEY_PEM` when set, else read
/// from the files named by `JWT_PRIVATE_KEY_FILE` / `JWT_PUBLIC_KEY_FILE`,
/// which default to the committed dev keypair. Additional verify-only
/// keys come from `JWT_ADDITIONAL_PUBLIC_KEY_FILES` /
/// `JWT_ADDITIONAL_PUBLIC_KEY_PEMS` (see [`additional_public_pems`]).
///
/// # Errors
///
/// Returns [`AuthError::Keys`] when a key is missing or not valid RSA.
pub fn load_keys() -> Result<AuthKeys, AuthError> {
    let private_pem = if let Ok(pem) = std::env::var("JWT_PRIVATE_KEY_PEM") {
        pem
    } else {
        let path = env_or("JWT_PRIVATE_KEY_FILE", "config/keys/jwt_private_dev.pem");
        std::fs::read_to_string(&path).map_err(|e| AuthError::Keys(format!("read {path}: {e}")))?
    };
    let public_pem = if let Ok(pem) = std::env::var("JWT_PUBLIC_KEY_PEM") {
        pem
    } else {
        let path = env_or("JWT_PUBLIC_KEY_FILE", "config/keys/jwt_public_dev.pem");
        std::fs::read_to_string(&path).map_err(|e| AuthError::Keys(format!("read {path}: {e}")))?
    };

    let additional = additional_public_pems()?;
    load_from(&private_pem, &public_pem, &additional)
}

/// Build an [`AuthKeys`] set from explicit PEM strings, reading the
/// issuer / audience / expiration policy from the environment (with the
/// usual defaults). Factored out of [`load_keys`] so tests can construct
/// a deterministic multi-key set without mutating process env or files.
///
/// `primary_private_pem` / `primary_public_pem` are the signing keypair;
/// `additional_public_pems` are extra verify-only public keys. Keys are
/// de-duplicated by `kid`, and the primary always wins the `kid` slot and
/// the JWKS-first position.
///
/// # Errors
///
/// Returns [`AuthError::Keys`] when any PEM is missing or not valid RSA.
pub fn load_from(
    primary_private_pem: &str,
    primary_public_pem: &str,
    additional_public_pems: &[String],
) -> Result<AuthKeys, AuthError> {
    let encoding = EncodingKey::from_rsa_pem(primary_private_pem.as_bytes())
        .map_err(|e| AuthError::Keys(format!("private key: {e}")))?;

    let primary = verify_key_from_public_pem(primary_public_pem)?;
    let kid = primary.kid.clone();

    // Build the JWKS (primary first) and the kid→decoding map, skipping
    // any additional key that duplicates the primary or a prior kid.
    let mut by_kid: HashMap<String, DecodingKey> = HashMap::new();
    let mut jwk_list: Vec<serde_json::Value> = Vec::new();

    jwk_list.push(primary.jwk);
    by_kid.insert(primary.kid, primary.decoding);

    for pem in additional_public_pems {
        let extra = verify_key_from_public_pem(pem)?;
        if by_kid.contains_key(&extra.kid) {
            continue; // already present (e.g. duplicate of primary)
        }
        jwk_list.push(extra.jwk);
        by_kid.insert(extra.kid, extra.decoding);
    }

    let issuer = env_or("JWT_ISSUER", "authentication-service");
    let audience = env_or("JWT_AUDIENCE", "main-x-service");
    let expiration = std::env::var("JWT_EXPIRATION")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_EXPIRATION_SECS);

    let jwks = serde_json::json!({ "keys": jwk_list });

    Ok(AuthKeys {
        encoding,
        by_kid,
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

/// Verify an RS256 token against the local key **set**, checking issuer,
/// audience, and expiry. The verifying key is selected by the token
/// header's `kid`, so a token signed by a key that has since rotated down
/// to "additional" (verify-only) still verifies until it expires. A token
/// whose `kid` is absent or matches no key in the set is rejected.
///
/// # Errors
///
/// Returns the underlying `jsonwebtoken` error on any validation failure,
/// including an `InvalidSignature` when the header `kid` is missing or
/// unknown (no key in the set can verify it).
pub fn verify_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let k = keys();
    let header = decode_header(token)?;
    let decoding = header
        .kid
        .as_deref()
        .and_then(|kid| k.by_kid.get(kid))
        .ok_or_else(|| {
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidSignature)
        })?;
    let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(std::slice::from_ref(&k.issuer));
    validation.set_audience(std::slice::from_ref(&k.audience));
    decode::<Claims>(token, decoding, &validation).map(|data| data.claims)
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

    // --- Key rotation (T-5) ---------------------------------------------
    //
    // These build deterministic key sets in-process via `load_from`
    // (no env mutation, no extra files), so they exercise the multi-key
    // path without disturbing the shared `keys()` singleton.

    // The committed dev keypair (the same one `keys()` resolves). Reading
    // it here lets a test build a multi-key set whose primary equals the
    // process primary, so tokens from `sign_access_token` are verifiable.
    fn dev_keypair() -> (String, String) {
        let private = std::fs::read_to_string("config/keys/jwt_private_dev.pem")
            .expect("committed dev private key");
        let public = std::fs::read_to_string("config/keys/jwt_public_dev.pem")
            .expect("committed dev public key");
        (private, public)
    }

    // A second, throwaway RSA keypair distinct from the dev keypair. Used
    // as an "additional" (rotated-out) key and as an unknown signer.
    const OTHER_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCb8A4bMels7eEL\n\
TCFMcXyJTMdQ0k2klAxkPjPRHdAtwzV+6uLjKBlI2NTubkUl0BcKLKpX1G9mDjci\n\
Qiga5frQ/QX9IedKGdU3aLo0BvpewAK2TmRwDe48i3d+uGWPbDcDd9DG0hEz6ugd\n\
CXeCcXekdcsX8e8bF9E4FYK+2DaI9xiVhMQS8xSY7Mt3Os/T8GBT27/JdKkzoX2Z\n\
ntYiOOhxwcwztS+W8y1qVD6b39hPCo2vbl+nyFdN6tz00L/qakJMV4AJjXOYhT5N\n\
XBYII92vmlznYNWNTtBymmaGhes7RbIb9AvgC2PJZdk57Sxl0tP71CsIX2P/RjPD\n\
AoExpOfDAgMBAAECggEAKwLNGUIslM+OJZwTiS66P3KufUPsh4sQWevwRes3uw+f\n\
Z0jpWOd8BeRM4xEGQJZDbJqCR6SAL4GXQntF7Zlmk5NevgHGdmFmtphL18Le9xh2\n\
BwvbVy74ebmsNYct+B/MksfPDa/ub8gIys2MKa4bZoDZCltAbNQmcJY6UGJ5tFAp\n\
ItKFgoA8wnxJroUGcw1r7B8WRyxBGxSqjYVmwtRyUcbea1gCVfCXGwkkVgv42Miu\n\
YY6C7y9e0zlwcAXdkZGTgfYlz/hiBWd2xAg+tGV2bwVBRlX3Io1qcNUEphOHDp+l\n\
iTf/E8DkLvln3J9DsBD6jscWE8lK8HDzLHPpT1EkSQKBgQDQ7KW2so/ZMPMvLo+R\n\
j61XUqvRW9sJgsQvZsukGt+dbq/YDJho6J0mu2OC8Sag2ZvLezGmGdjMkUHIvza/\n\
3KpHau4vPG0LByqZv2sF+9XAOLo5YAVPZlZFb8egYH7X2bOmVaupyGKSsmcMUsHa\n\
x7jZf84vIY033RcLhirjDDg6ywKBgQC/Evw1CXby8WVKBPNB8pr7VKE6oy6DK76d\n\
TESJiirq1Z8am1WKxOzvo9OlZPibfuZKeY/XiF2fj5YHWc1xR7S7+OPMlwZdVx64\n\
h7Iv3j9yFS7jwX1AVxmOB/b48ki+QEItTEiQJqEQXiFEGr0MgI6fTj/opKEltv5L\n\
6cfdqOCP6QKBgQCu4LchQzvnV/Lm1nl0JSi6REfvuYyR3HRtHQVuOtRcig8EsB5P\n\
Cg6pIgd8znBACYY//8GiQFZZfWjsKSoh1QpvN1FiFplLttbw1Oo3mwHjoVg3uGkZ\n\
ehbSjmsxkjP6Z47ZtzI2rrXcBxr8lLURdUYEQNeMWfBEB3tHuSli3ZKfmwKBgQCn\n\
YfdEcuUbz7H+lLWQqPlxgGK5HmhJilGyNDS6FCqii76ULU1Tgk1ZZLesZPaQKSuO\n\
RE1o71GszLkN+XJKcRl3rYHJIOf3brE/z8edvWDxDHOGG2MgsOx3Cq0kygJFf785\n\
NWE/vkdMMlmL8qx3vkqybXb40vdENbkxQTvQBvepuQKBgGHKxTj314X4xObeQ5BY\n\
yteRm758Id7MoieW0dYZC62a05STaiyCB5ulVCijNa66616uxyAMhquKru9xT1Bt\n\
zGUi4GKlyqqAH7webJPHDR58z5Jj4XqAblYzRFY3nqSAd6lxdjMdIftQi0xrG6+x\n\
UOsbyJ6S44rLeDtZ9KGxR0gS\n\
-----END PRIVATE KEY-----\n";

    const OTHER_PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----\n\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAm/AOGzHpbO3hC0whTHF8\n\
iUzHUNJNpJQMZD4z0R3QLcM1furi4ygZSNjU7m5FJdAXCiyqV9RvZg43IkIoGuX6\n\
0P0F/SHnShnVN2i6NAb6XsACtk5kcA3uPIt3frhlj2w3A3fQxtIRM+roHQl3gnF3\n\
pHXLF/HvGxfROBWCvtg2iPcYlYTEEvMUmOzLdzrP0/BgU9u/yXSpM6F9mZ7WIjjo\n\
ccHMM7UvlvMtalQ+m9/YTwqNr25fp8hXTerc9NC/6mpCTFeACY1zmIU+TVwWCCPd\n\
r5pc52DVjU7QcppmhoXrO0WyG/QL4AtjyWXZOe0sZdLT+9QrCF9j/0YzwwKBMaTn\n\
wwIDAQAB\n\
-----END PUBLIC KEY-----\n";

    // Sign a token with an arbitrary keypair + kid (mirrors how
    // `sign_access_token` builds the header) so tests can produce tokens
    // for primary, additional, and unknown signers deterministically.
    fn sign_with(private_pem: &str, kid: &str, k: &AuthKeys) -> String {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            iss: k.issuer.clone(),
            aud: k.audience.clone(),
            exp: now + 3600,
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
        };
        let mut header = Header::new(jsonwebtoken::Algorithm::RS256);
        header.kid = Some(kid.to_string());
        let enc = EncodingKey::from_rsa_pem(private_pem.as_bytes()).expect("encoding key");
        encode(&header, &claims, &enc).expect("encode")
    }

    // Verify against an explicit key set (the rotation tests build their
    // own `AuthKeys`, so they can't use the singleton `verify_token`).
    fn verify_with(k: &AuthKeys, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let header = decode_header(token)?;
        let decoding = header
            .kid
            .as_deref()
            .and_then(|kid| k.by_kid.get(kid))
            .ok_or_else(|| {
                jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidSignature)
            })?;
        let mut validation = Validation::new(jsonwebtoken::Algorithm::RS256);
        validation.set_issuer(std::slice::from_ref(&k.issuer));
        validation.set_audience(std::slice::from_ref(&k.audience));
        decode::<Claims>(token, decoding, &validation).map(|data| data.claims)
    }

    #[test]
    fn no_additional_keys_is_backward_compatible_single_key() {
        let (private, public) = dev_keypair();
        let set = load_from(&private, &public, &[]).expect("load");
        // One key only, and its kid equals the singleton's kid.
        assert_eq!(set.key_count(), 1);
        assert_eq!(set.jwks["keys"].as_array().unwrap().len(), 1);
        assert_eq!(set.kid, keys().kid);
        assert_eq!(set.jwks["keys"][0]["kid"].as_str().unwrap(), set.kid);
    }

    #[test]
    fn jwks_publishes_all_keys_primary_first() {
        let (private, public) = dev_keypair();
        let set = load_from(&private, &public, &[OTHER_PUBLIC_PEM.to_string()]).expect("load");
        assert_eq!(set.key_count(), 2);
        let arr = set.jwks["keys"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        // Primary is first; the additional key is present under its own kid.
        assert_eq!(arr[0]["kid"].as_str().unwrap(), set.kid);
        let other = verify_key_from_public_pem(OTHER_PUBLIC_PEM).expect("derive");
        assert_eq!(arr[1]["kid"].as_str().unwrap(), other.kid);
        assert_ne!(set.kid, other.kid);
    }

    #[test]
    fn token_from_now_additional_key_still_verifies() {
        // Simulate a rotation: the OTHER key used to be primary, and is
        // now an additional (verify-only) key alongside the new primary.
        let (private, public) = dev_keypair();
        let set = load_from(&private, &public, &[OTHER_PUBLIC_PEM.to_string()]).expect("load");
        let other = verify_key_from_public_pem(OTHER_PUBLIC_PEM).expect("derive");

        // A token signed by the (now additional) OTHER key verifies.
        let token = sign_with(OTHER_PRIVATE_PEM, &other.kid, &set);
        let claims = verify_with(&set, &token).expect("additional-key token verifies");
        assert_eq!(claims.email, "alice@example.com");

        // A token signed by the primary also verifies.
        let primary_token = sign_with(&private, &set.kid, &set);
        assert!(verify_with(&set, &primary_token).is_ok());
    }

    #[test]
    fn unknown_kid_token_is_rejected() {
        let (private, public) = dev_keypair();
        // Set with only the primary; OTHER is NOT in the set.
        let set = load_from(&private, &public, &[]).expect("load");
        let token = sign_with(OTHER_PRIVATE_PEM, "some-unknown-kid", &set);
        assert!(verify_with(&set, &token).is_err());
    }

    #[test]
    fn duplicate_additional_key_is_deduplicated() {
        let (private, public) = dev_keypair();
        // Passing the primary's own public key as "additional" must not
        // produce a second JWKS entry.
        let set = load_from(&private, &public, std::slice::from_ref(&public)).expect("load");
        assert_eq!(set.key_count(), 1);
        assert_eq!(set.jwks["keys"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn split_pems_separates_concatenated_blocks() {
        let blob = format!("{OTHER_PUBLIC_PEM}\n{OTHER_PUBLIC_PEM}");
        let pems = split_pems(&blob);
        assert_eq!(pems.len(), 2);
        for pem in &pems {
            assert!(verify_key_from_public_pem(pem).is_ok());
        }
    }
}
