//! PASETO v4.public token issuance + verification + public-key publication.
//!
//! This crate is the federation's single auth provider. The durable
//! session is a server-side cookie session (see the auth controller and
//! [`agents/share/authentication-sessions.md`]); from that session this
//! module mints short-lived **PASETO v4.public** (Ed25519) access tokens,
//! which every other service verifies *offline* against the public key set
//! published at `/.well-known/paseto-keys`. There is no shared secret and
//! no per-request introspection call.
//!
//! This supersedes the crate's prior RS256-JWT + JWKS design: PASETO is a
//! versioned, misuse-resistant format with no algorithm agility, so the
//! `alg=none` / "alg confusion" class of JWT attacks does not exist.
//!
//! ## Key rotation (zero-downtime)
//!
//! [`AuthKeys`] is a **key set**: one *primary* signing key plus zero or
//! more *additional* verify-only public keys. [`sign_access_token`] always
//! signs with the primary and stamps the primary's `kid` in the token
//! footer; [`verify_token`] selects the verifying key by the footer `kid`,
//! so a token signed by a key that has since rotated down to "additional"
//! still verifies until it expires. The published key set carries every
//! key (primary first). Additional keys load from
//! `TOKEN_ADDITIONAL_PUBLIC_KEYS` (comma-separated base64url Ed25519 public
//! keys); unset ⇒ just the primary.
//!
//! [`agents/share/authentication-sessions.md`]: https://github.com/sixarm/main-x-service

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{
    Footer, ImplicitAssertion, Key, Paseto, PasetoAsymmetricPrivateKey, PasetoAsymmetricPublicKey,
    Public, UntrustedToken, V4,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Default access-token lifetime (seconds). Deliberately short (~5 min):
/// the cookie session is the durable thing, and with offline verification a
/// revoked session's already-minted tokens remain valid at peers until they
/// expire, so the window is kept small.
const DEFAULT_EXPIRATION_SECS: i64 = 300;

/// Built-in development signing seed (32 bytes). Used only when no
/// `TOKEN_PRIVATE_KEY_SEED` env / file is configured, so `cargo test` and
/// local dev run offline and deterministically. **Never** relied on in
/// production — set `TOKEN_PRIVATE_KEY_SEED` there. Mirrors the prior
/// committed dev keypair.
const DEV_SEED: [u8; 32] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32,
];

/// Resolved signing/verification material plus the published key set.
///
/// Holds a **set**: exactly one primary signing key (used by
/// [`sign_access_token`]) and zero or more additional verify-only public
/// keys whose still-live tokens must keep verifying. [`verify_token`]
/// selects among them by the token footer `kid`.
pub struct AuthKeys {
    /// Primary Ed25519 signing keypair bytes (64-byte seed||public), the
    /// form `rusty_paseto` expects for `PasetoAsymmetricPrivateKey`.
    signing_keypair: [u8; 64],
    /// Verification public keys (raw 32 bytes) keyed by `kid`: the primary
    /// plus any additional.
    by_kid: HashMap<String, [u8; 32]>,
    /// `kid` of the **primary** key — stamped into every token footer and
    /// the first entry of the published key set.
    pub kid: String,
    /// `iss` claim and key-set issuer.
    pub issuer: String,
    /// `aud` claim — the federation audience.
    pub audience: String,
    /// Access-token lifetime in seconds.
    pub expiration: i64,
    /// Pre-rendered key-set document served at `/.well-known/paseto-keys`
    /// (every key, primary first), in the OKP/Ed25519 JWK form the
    /// `authentication-verifier` crate parses.
    pub paseto_keys: serde_json::Value,
}

impl AuthKeys {
    /// Number of verification keys in the set (1 primary + additional).
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.by_kid.len()
    }
}

/// Access-token claims. `sub` carries the user's public id (`pid`). The
/// field set is a byte-for-byte contract with the peer-side
/// `authentication_verifier::Claims`, pinned by the cross-crate contract
/// test (`tests/sign_verify_contract.rs`).
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
    /// Not-before (unix seconds); omitted from the wire form when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,
    /// Session id — the originating server-side session (`sessions.jid`),
    /// enabling correlation and revocation.
    pub sid: String,
    /// Granted scopes, if any.
    #[serde(default)]
    pub scope: Vec<String>,
    /// Granted roles, if any.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Subject attributes for ABAC authorization — a string→strings map
    /// minted from the user's assigned attributes (e.g.
    /// `access: ["write"]`, `dept: ["cardiology"]`, `svc: ["true"]` for
    /// machine peers). Multi-valued keys mean "has each of these values";
    /// policies match set-membership; unknown attributes are inert
    /// (forward-compatible). Absent on the wire (old tokens) ⇒ empty map
    /// — no re-issue needed. Evaluated by the peer-side
    /// `authentication_verifier::abac` engine per
    /// `agents/share/authorization-attributes.md` §2–§3.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attrs: BTreeMap<String, Vec<String>>,
}

/// Errors from key loading or token handling.
#[derive(Debug)]
pub enum AuthError {
    /// Key material could not be loaded or parsed.
    Keys(String),
    /// Token signing failed.
    Sign(String),
    /// Token verification failed (bad token, unknown `kid`, or rejected
    /// claim).
    Verify(String),
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Keys(m) => write!(f, "paseto key error: {m}"),
            AuthError::Sign(m) => write!(f, "paseto sign error: {m}"),
            AuthError::Verify(m) => write!(f, "paseto verify error: {m}"),
        }
    }
}

impl std::error::Error for AuthError {}

static KEYS: OnceLock<AuthKeys> = OnceLock::new();

/// Process-wide accessor for the resolved key material. Initialised on
/// first use; a misconfiguration is a fatal boot error, so it panics with
/// actionable context rather than degrading silently.
///
/// # Panics
///
/// Panics if key material cannot be loaded (see [`load_keys`]).
pub fn keys() -> &'static AuthKeys {
    KEYS.get_or_init(|| {
        load_keys().unwrap_or_else(|e| {
            panic!(
                "failed to load PASETO keys ({e}). Set TOKEN_PRIVATE_KEY_SEED \
                 (base64url 32-byte Ed25519 seed) or TOKEN_PRIVATE_KEY_FILE, \
                 or rely on the built-in dev seed for local development."
            )
        })
    })
}

fn env_or(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

/// `kid` for an Ed25519 public key: `base64url(SHA-256(public bytes))`,
/// stable across restarts and identical in token footers and the key set.
fn kid_for(public: &[u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(public))
}

/// Render the OKP/Ed25519 JWK the key set publishes for one public key.
fn jwk_for(kid: &str, public: &[u8; 32]) -> serde_json::Value {
    serde_json::json!({
        "kty": "OKP",
        "crv": "Ed25519",
        "use": "sig",
        "kid": kid,
        "x": URL_SAFE_NO_PAD.encode(public),
    })
}

/// Resolve the primary signing seed: `TOKEN_PRIVATE_KEY_SEED` (inline
/// base64url 32 bytes) → `TOKEN_PRIVATE_KEY_FILE` (a file holding the same)
/// → the built-in [`DEV_SEED`].
fn load_seed() -> Result<[u8; 32], AuthError> {
    let b64 = if let Ok(seed) = std::env::var("TOKEN_PRIVATE_KEY_SEED") {
        Some(seed)
    } else if let Ok(path) = std::env::var("TOKEN_PRIVATE_KEY_FILE") {
        Some(
            std::fs::read_to_string(&path)
                .map_err(|e| AuthError::Keys(format!("read {path}: {e}")))?,
        )
    } else {
        None
    };
    match b64 {
        Some(s) => {
            let bytes = URL_SAFE_NO_PAD
                .decode(s.trim())
                .map_err(|e| AuthError::Keys(format!("seed base64url: {e}")))?;
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| AuthError::Keys("seed is not 32 bytes".to_string()))
        }
        None => Ok(DEV_SEED),
    }
}

/// Resolve additional verify-only public keys from
/// `TOKEN_ADDITIONAL_PUBLIC_KEYS` (comma-separated base64url 32-byte
/// Ed25519 public keys). Unset/empty ⇒ none.
fn additional_public_keys() -> Result<Vec<[u8; 32]>, AuthError> {
    let mut out = Vec::new();
    if let Ok(list) = std::env::var("TOKEN_ADDITIONAL_PUBLIC_KEYS") {
        for entry in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let bytes = URL_SAFE_NO_PAD
                .decode(entry)
                .map_err(|e| AuthError::Keys(format!("additional key base64url: {e}")))?;
            let key: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| AuthError::Keys("additional key is not 32 bytes".to_string()))?;
            out.push(key);
        }
    }
    Ok(out)
}

/// Resolve key material from the environment (or the dev seed) and build
/// the signing key, verification set, and published key-set document.
///
/// # Errors
///
/// Returns [`AuthError::Keys`] when a key is missing or not a valid 32-byte
/// Ed25519 value.
pub fn load_keys() -> Result<AuthKeys, AuthError> {
    let seed = load_seed()?;
    let additional = additional_public_keys()?;
    build_keys(&seed, &additional)
}

/// Build an [`AuthKeys`] set from an explicit primary seed plus additional
/// verify-only public keys, reading issuer / audience / expiration from the
/// environment. Factored out so tests can construct a deterministic set
/// without mutating process env.
///
/// # Errors
///
/// Returns [`AuthError::Keys`] if a key is invalid.
pub fn build_keys(seed: &[u8; 32], additional: &[[u8; 32]]) -> Result<AuthKeys, AuthError> {
    let signing = SigningKey::from_bytes(seed);
    let primary_public = signing.verifying_key().to_bytes();
    let kid = kid_for(&primary_public);

    let mut by_kid: HashMap<String, [u8; 32]> = HashMap::new();
    let mut jwk_list: Vec<serde_json::Value> = Vec::new();

    jwk_list.push(jwk_for(&kid, &primary_public));
    by_kid.insert(kid.clone(), primary_public);

    for public in additional {
        let extra_kid = kid_for(public);
        if by_kid.contains_key(&extra_kid) {
            continue; // duplicate of the primary or a prior additional key
        }
        jwk_list.push(jwk_for(&extra_kid, public));
        by_kid.insert(extra_kid, *public);
    }

    let issuer = env_or("TOKEN_ISSUER", "authentication-service");
    let audience = env_or("TOKEN_AUDIENCE", "main-x-service");
    let expiration = std::env::var("TOKEN_EXPIRATION")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(DEFAULT_EXPIRATION_SECS);

    Ok(AuthKeys {
        signing_keypair: signing.to_keypair_bytes(),
        by_kid,
        kid,
        issuer,
        audience,
        expiration,
        paseto_keys: serde_json::json!({ "keys": jwk_list }),
    })
}

/// Mint a PASETO `v4.public` access token for a user, bound to session
/// `sid` and carrying the subject's ABAC `attrs` map (empty ⇒ the claim
/// is omitted from the wire form). Returns the token, the `sid` (echoed
/// for the caller's convenience), and its expiry (unix seconds).
///
/// # Errors
///
/// Returns [`AuthError::Sign`] when signing fails.
pub fn sign_access_token(
    user_pid: &str,
    email: &str,
    name: &str,
    sid: &str,
    attrs: BTreeMap<String, Vec<String>>,
) -> Result<(String, String, i64), AuthError> {
    let k = keys();
    let now = chrono::Utc::now().timestamp();
    let exp = now + k.expiration;
    let claims = Claims {
        sub: user_pid.to_string(),
        email: email.to_string(),
        name: name.to_string(),
        iss: k.issuer.clone(),
        aud: k.audience.clone(),
        exp,
        iat: now,
        nbf: None,
        sid: sid.to_string(),
        scope: Vec::new(),
        roles: Vec::new(),
        attrs,
    };
    let token = mint(&k.signing_keypair, &k.kid, &claims)?;
    Ok((token, sid.to_string(), exp))
}

/// Sign `claims` into a `v4.public` token with `kid` in the footer, using
/// the given 64-byte Ed25519 keypair.
fn mint(keypair: &[u8; 64], kid: &str, claims: &Claims) -> Result<String, AuthError> {
    let key = Key::<64>::from(keypair);
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let payload = serde_json::to_string(claims).map_err(|e| AuthError::Sign(e.to_string()))?;
    let footer = format!(r#"{{"kid":"{kid}"}}"#);
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(rusty_paseto::core::Payload::from(payload.as_str()));
    builder.set_footer(Footer::from(footer.as_str()));
    builder
        .try_sign(&private)
        .map_err(|e| AuthError::Sign(e.to_string()))
}

/// Verify a `v4.public` token against the local key **set** (selecting the
/// key by the footer `kid`), enforcing issuer, audience, expiry, and
/// not-before. Mirrors the peer-side `authentication-verifier`.
///
/// # Errors
///
/// Returns [`AuthError::Verify`] on any structural, signature, or claim
/// failure (including a missing/unknown footer `kid`).
pub fn verify_token(token: &str) -> Result<Claims, AuthError> {
    let k = keys();
    verify_with(k, token)
}

/// Verify against an explicit key set (used by [`verify_token`] and tests).
fn verify_with(k: &AuthKeys, token: &str) -> Result<Claims, AuthError> {
    if !token.starts_with("v4.public.") {
        return Err(AuthError::Verify("not a v4.public token".to_string()));
    }
    let untrusted =
        UntrustedToken::try_parse(token).map_err(|e| AuthError::Verify(e.to_string()))?;
    let footer = untrusted
        .footer_str()
        .map_err(|e| AuthError::Verify(e.to_string()))?
        .ok_or_else(|| AuthError::Verify("token footer has no kid".to_string()))?;
    let footer_json: serde_json::Value =
        serde_json::from_str(&footer).map_err(|e| AuthError::Verify(format!("footer: {e}")))?;
    let kid = footer_json
        .get("kid")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AuthError::Verify("token footer has no kid".to_string()))?;
    let public = k
        .by_kid
        .get(kid)
        .ok_or_else(|| AuthError::Verify(format!("no key for kid {kid:?}")))?;
    let key = Key::<32>::from(public);
    let public_key = PasetoAsymmetricPublicKey::<V4, Public>::from(&key);
    let payload = Paseto::<V4, Public>::try_verify(
        token,
        &public_key,
        Footer::from(footer.as_str()),
        Option::<ImplicitAssertion>::None,
    )
    .map_err(|e| AuthError::Verify(e.to_string()))?;
    let claims: Claims =
        serde_json::from_str(&payload).map_err(|e| AuthError::Verify(format!("payload: {e}")))?;
    if claims.iss != k.issuer {
        return Err(AuthError::Verify("issuer mismatch".to_string()));
    }
    if claims.aud != k.audience {
        return Err(AuthError::Verify("audience mismatch".to_string()));
    }
    let now = chrono::Utc::now().timestamp();
    if let Some(nbf) = claims.nbf
        && now < nbf
    {
        return Err(AuthError::Verify("token not yet valid (nbf)".to_string()));
    }
    if now >= claims.exp {
        return Err(AuthError::Verify("token expired".to_string()));
    }
    Ok(claims)
}

/// Axum extractor for a verified bearer token. Pulls `Authorization:
/// Bearer <paseto>`, verifies it, and yields the [`Claims`]. Stateless — it
/// does not touch the database; revocation (signout) is enforced separately
/// by handlers that consult the `sessions` table.
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

    // Build a deterministic key set from the dev seed (no env, no files).
    fn dev_keys() -> AuthKeys {
        build_keys(&DEV_SEED, &[]).expect("build dev keys")
    }

    // A second, throwaway seed distinct from the dev seed.
    const OTHER_SEED: [u8; 32] = [
        9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
        9, 9,
    ];

    fn claims_for(k: &AuthKeys, sid: &str, exp_offset: i64) -> Claims {
        let now = chrono::Utc::now().timestamp();
        Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            iss: k.issuer.clone(),
            aud: k.audience.clone(),
            exp: now + exp_offset,
            iat: now,
            nbf: None,
            sid: sid.to_string(),
            scope: Vec::new(),
            roles: Vec::new(),
            attrs: BTreeMap::new(),
        }
    }

    #[test]
    fn published_key_set_is_one_ed25519_key() {
        let k = dev_keys();
        let key = &k.paseto_keys["keys"][0];
        assert_eq!(key["kty"], "OKP");
        assert_eq!(key["crv"], "Ed25519");
        assert_eq!(key["use"], "sig");
        assert_eq!(key["kid"].as_str(), Some(k.kid.as_str()));
        assert!(key["x"].as_str().is_some_and(|s| !s.is_empty()));
    }

    #[test]
    fn mint_then_verify_round_trips_claims() {
        let k = dev_keys();
        let claims = claims_for(&k, "session-abc", 3600);
        let token = mint(&k.signing_keypair, &k.kid, &claims).expect("mint");
        let got = verify_with(&k, &token).expect("verify");
        assert_eq!(got.sub, claims.sub);
        assert_eq!(got.email, "alice@example.com");
        assert_eq!(got.iss, k.issuer);
        assert_eq!(got.aud, k.audience);
        assert_eq!(got.sid, "session-abc");
    }

    #[test]
    fn attrs_claim_round_trips_mint_to_verify() {
        let k = dev_keys();
        let mut claims = claims_for(&k, "session-abc", 3600);
        claims
            .attrs
            .insert("access".to_string(), vec!["write".to_string()]);
        claims.attrs.insert(
            "dept".to_string(),
            vec!["cardiology".to_string(), "oncology".to_string()],
        );
        let token = mint(&k.signing_keypair, &k.kid, &claims).expect("mint");
        let got = verify_with(&k, &token).expect("verify");
        assert_eq!(got.attrs, claims.attrs);
    }

    #[test]
    fn empty_attrs_claim_is_omitted_from_the_payload() {
        // skip_serializing_if: an empty map never reaches the wire, so
        // pre-ABAC verifiers see the exact pre-0.3 payload shape.
        let k = dev_keys();
        let claims = claims_for(&k, "s", 3600);
        let payload = serde_json::to_value(&claims).expect("serialize");
        assert!(payload.get("attrs").is_none());
        // …and it still deserializes back to an empty map.
        let got: Claims = serde_json::from_value(payload).expect("deserialize");
        assert!(got.attrs.is_empty());
    }

    #[test]
    fn expired_token_is_rejected() {
        let k = dev_keys();
        let claims = claims_for(&k, "s", -10_000);
        let token = mint(&k.signing_keypair, &k.kid, &claims).expect("mint");
        assert!(verify_with(&k, &token).is_err());
    }

    #[test]
    fn tampered_token_is_rejected() {
        let k = dev_keys();
        let claims = claims_for(&k, "s", 3600);
        let token = mint(&k.signing_keypair, &k.kid, &claims).expect("mint");
        let mut parts: Vec<&str> = token.split('.').collect();
        let mut payload = parts[2].to_string();
        let last = payload.len() - 1;
        let swapped = if &payload[last..] == "A" { "B" } else { "A" };
        payload.replace_range(last.., swapped);
        parts[2] = &payload;
        let tampered = parts.join(".");
        assert!(verify_with(&k, &tampered).is_err());
    }

    #[test]
    fn garbage_token_is_rejected() {
        let k = dev_keys();
        assert!(verify_with(&k, "not.a.paseto").is_err());
        assert!(verify_with(&k, "").is_err());
        assert!(verify_with(&k, "v2.public.aaaa").is_err());
    }

    #[test]
    fn additional_key_token_still_verifies_after_rotation() {
        // OTHER used to be primary; now it is an additional verify-only key
        // alongside the dev primary.
        let other_public = SigningKey::from_bytes(&OTHER_SEED)
            .verifying_key()
            .to_bytes();
        let k = build_keys(&DEV_SEED, &[other_public]).expect("build");
        assert_eq!(k.key_count(), 2);

        // A token signed by the (now additional) OTHER key verifies.
        let other_keypair = SigningKey::from_bytes(&OTHER_SEED).to_keypair_bytes();
        let other_kid = kid_for(&other_public);
        let claims = claims_for(&k, "s", 3600);
        let token = mint(&other_keypair, &other_kid, &claims).expect("mint");
        assert!(verify_with(&k, &token).is_ok());

        // A token signed by the primary also verifies.
        let primary = mint(&k.signing_keypair, &k.kid, &claims).expect("mint");
        assert!(verify_with(&k, &primary).is_ok());
    }

    #[test]
    fn unknown_kid_token_is_rejected() {
        // Set with only the primary; OTHER is not in the set.
        let k = build_keys(&DEV_SEED, &[]).expect("build");
        let other_keypair = SigningKey::from_bytes(&OTHER_SEED).to_keypair_bytes();
        let other_kid = kid_for(
            &SigningKey::from_bytes(&OTHER_SEED)
                .verifying_key()
                .to_bytes(),
        );
        let claims = claims_for(&k, "s", 3600);
        let token = mint(&other_keypair, &other_kid, &claims).expect("mint");
        assert!(verify_with(&k, &token).is_err());
    }

    #[test]
    fn duplicate_additional_key_is_deduplicated() {
        let primary_public = SigningKey::from_bytes(&DEV_SEED).verifying_key().to_bytes();
        // Passing the primary's own public key as "additional" must not add
        // a second key-set entry.
        let k = build_keys(&DEV_SEED, &[primary_public]).expect("build");
        assert_eq!(k.key_count(), 1);
        assert_eq!(k.paseto_keys["keys"].as_array().unwrap().len(), 1);
    }
}
