//! Offline PASETO v4.public token verification against the
//! authentication-service's published Ed25519 keys.
//!
//! # The offline verification model
//!
//! The [`authentication-service`] is the federation's single auth
//! provider. A user session lives server-side (a Postgres-backed cookie
//! session); from that session the service mints short-lived **PASETO
//! v4.public** access tokens and publishes its **Ed25519 public keys** at
//! `/.well-known/paseto-keys`. Every other service verifies those tokens
//! *offline*: fetch the key set once at boot, build a [`Verifier`], then
//! call [`Verifier::verify`] per request. There is no shared secret and
//! no per-request introspection call.
//!
//! "Offline" is the key property and the reason this crate exists.
//! Because v4.public is *asymmetric* (Ed25519), the signer (auth-service)
//! holds the private key and the verifiers (every peer service) hold only
//! the public key. A peer can therefore confirm a token's authenticity
//! with no network round-trip on the hot path, no shared symmetric secret
//! to distribute, and no introspection endpoint to depend on for
//! availability. The only network interaction is the one-time (or rare)
//! key fetch, available out of band or via the optional
//! [`fetch`](crate#features) feature.
//!
//! This replaces the crate's previous RS256-JWT + JWKS design (≤ 0.1.x):
//! PASETO is a versioned, misuse-resistant token format with no algorithm
//! agility, so the "alg confusion" / `alg=none` class of JWT attacks does
//! not exist. See [`agents/share/authentication-sessions.md`] in the
//! monorepo for the family-wide design.
//!
//! # Authorization (ABAC)
//!
//! Since 0.3 the crate is also the family's shared **authorization**
//! foundation: verified [`Claims`] carry the subject's attributes in the
//! [`attrs`](Claims::attrs) claim, and the [`abac`] module provides the
//! pure policy engine ([`Policy`], [`Rule`], [`Action`],
//! [`Policy::evaluate`] → [`Decision`]) that the nine entity services
//! call from their blanket `/api/*` guards. See
//! [`agents/share/authorization-attributes.md`] for the design.
//!
//! # Security properties
//!
//! - **Asymmetric trust.** Verifiers never possess signing material, so a
//!   compromised peer cannot mint tokens — it can only verify them.
//! - **No algorithm agility.** The token header is the literal string
//!   `v4.public`; there is no `alg` field to downgrade and no `none`.
//! - **Key selection by `kid`.** The verifying key is chosen by the
//!   token's (authenticated) footer `kid`; a forged or stale `kid` simply
//!   matches no known key.
//! - **Issuer / audience / expiry enforcement.** Beyond a valid
//!   signature, every token must carry the expected `iss`, the expected
//!   `aud`, an unexpired `exp`, and (if present) a satisfied `nbf`.
//!
//! # Features
//!
//! - `fetch` (off by default) — adds [`Verifier::from_paseto_keys_url`],
//!   which pulls the key set over HTTPS via `reqwest` (rustls). With the
//!   feature off the crate does no I/O and the caller supplies the key set
//!   as a [`serde_json::Value`].
//!
//! # Example
//!
//! ```no_run
//! # use authentication_verifier::Verifier;
//! // Normally the keys come from the auth-service; an empty set here
//! // keeps the doctest offline and dependency-free.
//! let keys: serde_json::Value = serde_json::json!({ "keys": [] });
//! let verifier = Verifier::from_paseto_keys_value(&keys, "authentication-service", "main-x-service")?;
//! let claims = verifier.verify("v4.public...")?;
//! println!("authenticated subject: {}", claims.sub);
//! # Ok::<(), authentication_verifier::VerifyError>(())
//! ```
//!
//! [`authentication-service`]: https://github.com/sixarm/authentication-service-with-loco
//! [`agents/share/authentication-sessions.md`]: https://github.com/sixarm/main-x-service
//! [`agents/share/authorization-attributes.md`]: https://github.com/sixarm/main-x-service

// Reject `unsafe` outright: this crate touches only safe, allocation-light
// code paths and a security library has no business reaching for `unsafe`.
#![forbid(unsafe_code)]
// Opt into clippy's pedantic lints to keep the public-facing library tidy.
#![warn(clippy::pedantic)]
// A published library must document every public item; fail the build if
// any item lacks a doc comment.
#![deny(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

// base64url (no padding) decodes the published key's `x` component.
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
// The PASETO v4.public verify primitive plus the key / footer wrapper
// types. `UntrustedToken` lets us read the (authenticated) footer to
// select a key *before* verifying the signature.
use rusty_paseto::core::{
    Footer, ImplicitAssertion, Key, Paseto, PasetoAsymmetricPublicKey, Public, UntrustedToken, V4,
};
// `serde` derives let `Claims` deserialize from the token payload (and
// serialize again, which the tests rely on to mint tokens).
use serde::{Deserialize, Serialize};

pub mod abac;

// Re-export the ABAC engine types at the crate root so callers can use
// `authentication_verifier::{Policy, Action, ...}` alongside `Verifier`
// and `Claims` without spelling the module path.
pub use abac::{Action, ActionPattern, Decision, Effect, Policy, ReloadablePolicy, Rule};

/// Verified token claims. Mirrors the auth-service `Claims` exactly so a
/// token signed there round-trips here. `sub` carries the user `pid`.
///
/// The field set is a contract with the auth-service: the service defines
/// an identical struct, and changing one without the other breaks token
/// round-tripping. `exp` / `iat` / `nbf` are unix seconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user `pid` (UUID string); the stable identifier a
    /// peer service keys its authorization on.
    pub sub: String,
    /// User email, surfaced for convenience at the edge; not used for
    /// authorization decisions.
    pub email: String,
    /// Human-readable display name carried alongside the subject.
    pub name: String,
    /// Issuer (`iss`) — the auth-service that minted the token. Checked
    /// against the verifier's configured issuer.
    pub iss: String,
    /// Audience (`aud`) — the intended recipient service. Checked against
    /// the verifier's configured audience so a token issued for one peer
    /// cannot be replayed against another.
    pub aud: String,
    /// Expiry (`exp`), unix seconds. Tokens at or past this instant are
    /// rejected. Issued ~5 minutes out (the session is the durable thing).
    pub exp: i64,
    /// Issued-at (`iat`), unix seconds — when the token was minted.
    pub iat: i64,
    /// Not-before (`nbf`), unix seconds. When present, tokens before this
    /// instant are rejected. Omitted from the wire form when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>,
    /// Session id (`sid`) — the originating server-side session, so a
    /// token can be correlated back to (and revoked with) its session.
    pub sid: String,
    /// Granted scopes, if any. Empty when the token carries none.
    ///
    /// **Deprecated for authorization** (kept on the wire for
    /// compatibility; removal is a future major): the ABAC guard ignores
    /// `scope` and decides from [`attrs`](Self::attrs) instead. See
    /// `agents/share/authorization-attributes.md` §3.
    #[serde(default)]
    pub scope: Vec<String>,
    /// Granted roles, if any. Empty when the token carries none.
    ///
    /// **Deprecated for authorization** (kept on the wire for
    /// compatibility; removal is a future major): the ABAC guard ignores
    /// `roles` and decides from [`attrs`](Self::attrs) instead — a role,
    /// where one is wanted, is just another attribute (`role=editor`).
    /// See `agents/share/authorization-attributes.md` §3.
    #[serde(default)]
    pub roles: Vec<String>,
    /// Subject attributes for ABAC authorization — a string→strings map
    /// minted by the auth-service from the user's assigned attributes
    /// (e.g. `access: ["write"]`, `dept: ["cardiology"]`,
    /// `svc: ["true"]` for machine peers). Multi-valued keys mean "has
    /// each of these values"; policies match set-membership; unknown
    /// attributes are inert (forward-compatible). Absent on the wire
    /// (old tokens) ⇒ empty map — no re-issue needed. Evaluated by the
    /// [`abac`] engine per `agents/share/authorization-attributes.md`
    /// §2–§3, alongside the pseudo-attributes `sub` and `email`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attrs: BTreeMap<String, Vec<String>>,
}

/// Failure modes for key-set loading and token verification.
///
/// Every fallible entry point returns this type, and every variant is a
/// *handled* outcome — the crate never panics on bad input, so a malformed
/// key set and a forged token are both ordinary `Err` values.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// The key-set document was missing or structurally invalid (no `keys`
    /// array, a key missing `kid` / `x`, or an `x` that is not a 32-byte
    /// base64url Ed25519 public key). Raised at load time, not per token.
    #[error("malformed key set: {0}")]
    Keys(String),
    /// The token was not a structurally valid `v4.public` token, or its
    /// footer could not be decoded as `{ "kid": ... }`. Distinct from a
    /// signature failure ([`Paseto`](Self::Paseto)).
    #[error("malformed token: {0}")]
    Malformed(String),
    /// The token footer carried no `kid`, so no key could be selected.
    /// All auth-service tokens stamp a footer `kid`, so this indicates a
    /// hand-built or non-conforming token.
    #[error("token footer has no kid")]
    MissingKid,
    /// No verification key matched the token's footer `kid` (stale cache,
    /// wrong issuer, or forgery). The wrapped `String` is the unmatched
    /// `kid`; on a legitimate stale cache a caller may refetch and retry.
    #[error("no verification key for kid {0:?}")]
    UnknownKid(String),
    /// PASETO parsing or Ed25519 signature verification failed. Carries the
    /// stringified underlying `rusty_paseto` error.
    #[error("token verification failed: {0}")]
    Paseto(String),
    /// The signature was valid but a registered claim did not satisfy the
    /// policy: wrong `iss`, wrong `aud`, expired `exp`, or unmet `nbf`.
    #[error("claim rejected: {0}")]
    Claim(String),
    /// Fetching the key set over HTTP failed (only with the `fetch`
    /// feature): transport error, non-2xx status, or undecodable body.
    #[cfg(feature = "fetch")]
    #[error("key set fetch failed: {0}")]
    Fetch(String),
}

/// A set of Ed25519 verification keys (indexed by `kid`) plus the issuer /
/// audience policy applied to every token. Construct once at boot, then
/// share behind an `Arc` and call [`verify`](Verifier::verify) per
/// request — verification is read-only and allocation-light.
///
/// `kid` is an opaque string assigned by the auth-service and carried in
/// each token's footer; the verifier indexes its keys by exactly that
/// `kid`, so key selection at verify time is a direct map lookup.
pub struct Verifier {
    /// Ed25519 public keys (raw 32 bytes) keyed by their `kid`. Populated
    /// at construction; never mutated, so a `Verifier` is safe to share
    /// immutably across threads.
    keys: HashMap<String, [u8; 32]>,
    /// Expected issuer (`iss`) enforced on every token.
    issuer: String,
    /// Expected audience (`aud`) enforced on every token.
    audience: String,
}

impl Verifier {
    /// Build a verifier from an in-memory key-set document, validating
    /// tokens against `issuer` (`iss`) and `audience` (`aud`).
    ///
    /// The document mirrors a JWK set restricted to Ed25519:
    /// `{ "keys": [ { "kty": "OKP", "crv": "Ed25519", "kid": "...",
    /// "x": "<base64url 32-byte public key>" }, ... ] }`. Entries whose
    /// `kty`/`crv` are not `OKP`/`Ed25519` are skipped. An empty key set is
    /// permitted — it yields a verifier that rejects every token with
    /// [`VerifyError::UnknownKid`], so a service can boot before its key
    /// source is reachable without panicking.
    ///
    /// # Errors
    ///
    /// [`VerifyError::Keys`] when the document lacks a `keys` array, an
    /// Ed25519 key is missing `kid` / `x`, or `x` is not a 32-byte
    /// base64url value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use authentication_verifier::Verifier;
    /// let keys = serde_json::json!({ "keys": [] });
    /// let verifier = Verifier::from_paseto_keys_value(&keys, "authentication-service", "main-x-service")?;
    /// assert_eq!(verifier.key_count(), 0);
    /// # Ok::<(), authentication_verifier::VerifyError>(())
    /// ```
    pub fn from_paseto_keys_value(
        keys_doc: &serde_json::Value,
        issuer: &str,
        audience: &str,
    ) -> Result<Self, VerifyError> {
        let entries = keys_doc
            .get("keys")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| VerifyError::Keys("missing \"keys\" array".to_string()))?;

        let mut keys = HashMap::new();
        for jwk in entries {
            // Ed25519 only: silently skip any other key type so a key set
            // may legitimately advertise keys this verifier does not use.
            let kty = jwk.get("kty").and_then(serde_json::Value::as_str);
            let crv = jwk.get("crv").and_then(serde_json::Value::as_str);
            if kty != Some("OKP") || crv != Some("Ed25519") {
                continue;
            }
            // For an Ed25519 key the `kid` (lookup key) and `x` (the
            // 32-byte public key, base64url) are both required; a missing
            // field is a malformed key set, not a skippable entry.
            let kid = jwk
                .get("kid")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| VerifyError::Keys("ed25519 jwk missing \"kid\"".to_string()))?;
            let x = jwk
                .get("x")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| VerifyError::Keys(format!("jwk {kid} missing \"x\"")))?;
            let bytes = URL_SAFE_NO_PAD
                .decode(x)
                .map_err(|err| VerifyError::Keys(format!("jwk {kid}: bad base64url x: {err}")))?;
            let key: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| VerifyError::Keys(format!("jwk {kid}: x is not 32 bytes")))?;
            keys.insert(kid.to_string(), key);
        }

        Ok(Self {
            keys,
            issuer: issuer.to_string(),
            audience: audience.to_string(),
        })
    }

    /// Number of Ed25519 verification keys loaded.
    ///
    /// A count of zero means no token can ever verify (every
    /// [`verify`](Self::verify) call returns [`VerifyError::UnknownKid`]),
    /// which usually signals a key set that failed to load.
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Verify a PASETO `v4.public` bearer token: select the key by the
    /// footer `kid`, check the Ed25519 signature, then enforce issuer,
    /// audience, expiry, and not-before.
    ///
    /// Steps run cheapest-rejection-first: confirm the `v4.public` header,
    /// read the (authenticated) footer for its `kid`, select the key, then
    /// perform the signature check and finally the claim policy.
    ///
    /// # Errors
    ///
    /// - [`VerifyError::Malformed`] if the token is not a structurally
    ///   valid `v4.public` token or its footer is not `{ "kid": ... }`.
    /// - [`VerifyError::MissingKid`] if the footer carries no `kid`.
    /// - [`VerifyError::UnknownKid`] if the `kid` matches no loaded key.
    /// - [`VerifyError::Paseto`] if the Ed25519 signature check fails.
    /// - [`VerifyError::Claim`] if `iss` / `aud` / `exp` / `nbf` do not
    ///   satisfy the policy.
    ///
    /// # Examples
    ///
    /// ```
    /// # use authentication_verifier::{Verifier, VerifyError};
    /// let keys = serde_json::json!({ "keys": [] });
    /// let verifier = Verifier::from_paseto_keys_value(&keys, "authentication-service", "main-x-service")?;
    /// assert!(verifier.verify("not.a.paseto").is_err());
    /// # Ok::<(), VerifyError>(())
    /// ```
    pub fn verify(&self, token: &str) -> Result<Claims, VerifyError> {
        // 1. Pin the version + purpose by the literal header. PASETO has no
        //    algorithm field, so this is the whole "alg" decision.
        if !token.starts_with("v4.public.") {
            return Err(VerifyError::Malformed("not a v4.public token".to_string()));
        }
        // 2. Parse the token without trusting it, and read its footer. The
        //    footer is authenticated (covered by the signature), so reading
        //    the `kid` here and feeding the same footer back to `try_verify`
        //    is safe: tampering with it fails the signature check in step 4.
        let untrusted =
            UntrustedToken::try_parse(token).map_err(|err| VerifyError::Paseto(err.to_string()))?;
        let footer = untrusted
            .footer_str()
            .map_err(|err| VerifyError::Paseto(err.to_string()))?
            .ok_or(VerifyError::MissingKid)?;
        let footer_json: serde_json::Value = serde_json::from_str(&footer)
            .map_err(|err| VerifyError::Malformed(format!("footer is not json: {err}")))?;
        let kid = footer_json
            .get("kid")
            .and_then(serde_json::Value::as_str)
            .ok_or(VerifyError::MissingKid)?;
        // 3. Select the published key for this `kid`. A miss means we hold
        //    no key for this signer (stale cache, wrong issuer, or forgery).
        let key_bytes = self
            .keys
            .get(kid)
            .ok_or_else(|| VerifyError::UnknownKid(kid.to_string()))?;
        let key = Key::<32>::from(key_bytes);
        let public_key = PasetoAsymmetricPublicKey::<V4, Public>::from(&key);
        // 4. Verify the Ed25519 signature over (header, payload, footer).
        let payload = Paseto::<V4, Public>::try_verify(
            token,
            &public_key,
            Footer::from(footer.as_str()),
            Option::<ImplicitAssertion>::None,
        )
        .map_err(|err| VerifyError::Paseto(err.to_string()))?;
        // 5. Reconstruct claims and apply the issuer/audience/expiry policy.
        let claims: Claims = serde_json::from_str(&payload)
            .map_err(|err| VerifyError::Malformed(format!("payload is not claims json: {err}")))?;
        self.check_claims(&claims)?;
        Ok(claims)
    }

    /// Apply the registered-claim policy: `iss`, `aud`, `exp`, and `nbf`.
    fn check_claims(&self, claims: &Claims) -> Result<(), VerifyError> {
        if claims.iss != self.issuer {
            return Err(VerifyError::Claim(format!(
                "issuer mismatch: expected {:?}, got {:?}",
                self.issuer, claims.iss
            )));
        }
        if claims.aud != self.audience {
            return Err(VerifyError::Claim(format!(
                "audience mismatch: expected {:?}, got {:?}",
                self.audience, claims.aud
            )));
        }
        let now = now_unix();
        if let Some(nbf) = claims.nbf
            && now < nbf
        {
            return Err(VerifyError::Claim("token not yet valid (nbf)".to_string()));
        }
        if now >= claims.exp {
            return Err(VerifyError::Claim("token expired (exp)".to_string()));
        }
        Ok(())
    }
}

/// Current unix time in seconds, saturating to `i64::MAX` if the clock is
/// before the epoch (which would make every token "expired").
fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_secs()).ok())
        .unwrap_or(i64::MAX)
}

/// HTTP-loading constructor, available only with the `fetch` feature.
///
/// Kept in its own `cfg`-gated `impl` so the default build pulls in no
/// HTTP stack and does no I/O whatsoever.
#[cfg(feature = "fetch")]
impl Verifier {
    /// Fetch the key set from `url` over HTTPS and build a verifier. Call
    /// once at boot; the auth-service rotates keys rarely, so a process can
    /// cache the result for its lifetime (or refetch on
    /// [`VerifyError::UnknownKid`] to pick up a rotation).
    ///
    /// # Errors
    ///
    /// [`VerifyError::Fetch`] on any transport / non-2xx / decode error, or
    /// [`VerifyError::Keys`] when the fetched body is not a valid key set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use authentication_verifier::Verifier;
    /// # async fn run() -> Result<(), authentication_verifier::VerifyError> {
    /// let verifier = Verifier::from_paseto_keys_url(
    ///     "https://auth.example.com/.well-known/paseto-keys",
    ///     "authentication-service",
    ///     "main-x-service",
    /// )
    /// .await?;
    /// # let _ = verifier;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_paseto_keys_url(
        url: &str,
        issuer: &str,
        audience: &str,
    ) -> Result<Self, VerifyError> {
        // SEC-V1: hard cap on the key-set body so a hostile endpoint can't
        // OOM the peer. A published key set is a few hundred bytes.
        const MAX_KEYS_BYTES: usize = 64 * 1024;
        // SEC-V1: only fetch the key set over TLS. A plaintext (`http://`) or
        // silently-downgraded fetch lets a network attacker inject their own
        // Ed25519 public key, which is full **token forgery**. Reject a
        // non-`https` URL outright, and forbid redirects so an `https` URL
        // cannot be bounced to `http`.
        if !url
            .trim()
            .get(..8)
            .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
        {
            return Err(VerifyError::Fetch(format!(
                "key-set URL must be https:// (refusing to fetch {url})"
            )));
        }
        let client = reqwest::Client::builder()
            // SEC-V1: bound boot time so a hung/slow key endpoint can't stall
            // startup indefinitely.
            .timeout(std::time::Duration::from_secs(10))
            // SEC-V1: no redirects, so an https→http (or cross-host) bounce
            // can't defeat the scheme check above.
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| VerifyError::Fetch(e.to_string()))?;
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|e| VerifyError::Fetch(e.to_string()))?
            .error_for_status()
            .map_err(|e| VerifyError::Fetch(e.to_string()))?;

        // SEC-V1: read the body with the hard size cap (above) so a hostile
        // endpoint can't OOM the peer with an unbounded response.
        let mut buf: Vec<u8> = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| VerifyError::Fetch(e.to_string()))?
        {
            if buf.len() + chunk.len() > MAX_KEYS_BYTES {
                return Err(VerifyError::Fetch(format!(
                    "key set exceeds the {MAX_KEYS_BYTES}-byte limit"
                )));
            }
            buf.extend_from_slice(&chunk);
        }
        let body: serde_json::Value =
            serde_json::from_slice(&buf).map_err(|e| VerifyError::Fetch(e.to_string()))?;
        Self::from_paseto_keys_value(&body, issuer, audience)
    }
}

/// A **hot-reloadable** [`Verifier`] holder for **key rotation**: the
/// active verifier (its published Ed25519 key set) can be swapped at
/// runtime — e.g. by a periodic re-fetch of `/.well-known/paseto-keys` —
/// **without a restart**, while the per-request verify path stays
/// lock-light.
///
/// It wraps an `Arc<Verifier>` behind an `RwLock` (the same shape as
/// [`ReloadablePolicy`](crate::ReloadablePolicy)). Per request a guard
/// calls [`current`](Self::current) — a brief read-lock returning a cheap
/// `Arc` clone it verifies against; a refresh calls
/// [`store`](Self::store) — a brief write-lock swapping the `Arc`. A
/// verification in flight during a refresh finishes against its snapshot.
/// Poison-safe: a panic elsewhere never makes `current`/`store` panic.
///
/// The **refresh trigger** (a periodic timer, a signal) is the service's
/// concern — this type only holds and swaps the value. A refresh should
/// keep the current verifier on a fetch failure (never swap to an empty
/// key set), so a transient auth-service outage cannot lock everyone out.
///
/// (No `Debug` — [`Verifier`] deliberately does not derive it, so its key
/// material never lands in a debug log.)
pub struct ReloadableVerifier {
    inner: std::sync::RwLock<std::sync::Arc<Verifier>>,
}

impl ReloadableVerifier {
    /// Wrap an initial verifier (e.g. the one built/fetched at boot).
    #[must_use]
    pub fn new(verifier: Verifier) -> Self {
        Self {
            inner: std::sync::RwLock::new(std::sync::Arc::new(verifier)),
        }
    }

    /// The currently active verifier — a cheap `Arc` clone taken under a
    /// brief read-lock. Verify against the returned snapshot; a
    /// concurrent [`store`](Self::store) does not affect it.
    #[must_use]
    pub fn current(&self) -> std::sync::Arc<Verifier> {
        self.inner
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Atomically replace the active verifier (a brief write-lock) — e.g.
    /// after re-fetching a rotated key set. New requests verify against
    /// the new key set; in-flight ones finish against their snapshot.
    pub fn store(&self, verifier: Verifier) {
        *self
            .inner
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = std::sync::Arc::new(verifier);
    }
}

/// Offline unit tests.
///
/// The whole suite runs without network access: a fixed Ed25519 keypair
/// plays the auth-service's signing role, a key set is derived from its
/// public half exactly as the service would publish it, and tokens are
/// minted locally so each verification path is exercised deterministically.
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use rusty_paseto::core::{PasetoAsymmetricPrivateKey, Payload};

    // A fixed 32-byte Ed25519 seed → deterministic keypair, used only to
    // exercise the verifier offline. Not used anywhere in production.
    const TEST_SEED: [u8; 32] = [
        7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        7, 7,
    ];
    const ISSUER: &str = "authentication-service";
    const AUDIENCE: &str = "main-x-service";
    const KID: &str = "test-key-1";

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&TEST_SEED)
    }

    // Build a key-set document from the test public key, mirroring exactly
    // how the auth-service publishes (kty, crv, kid, x).
    fn test_keys() -> serde_json::Value {
        let public = signing_key().verifying_key().to_bytes();
        let x = URL_SAFE_NO_PAD.encode(public);
        serde_json::json!({
            "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig", "kid": KID, "x": x }]
        })
    }

    // Mint a v4.public token with the given footer kid and claims, using
    // the test private key — the inverse of what the verifier does.
    fn sign(kid: &str, claims: &Claims) -> String {
        let payload = serde_json::to_string(claims).expect("serialize claims");
        sign_payload(kid, &payload)
    }

    // Mint a v4.public token from a raw JSON payload string, so tests can
    // exercise wire forms `Claims` itself would not serialize (e.g. a
    // pre-0.3 token with no `attrs` member at all).
    fn sign_payload(kid: &str, payload: &str) -> String {
        let keypair = signing_key().to_keypair_bytes(); // [u8; 64]
        let key = Key::<64>::from(keypair);
        let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
        let footer = format!(r#"{{"kid":"{kid}"}}"#);
        let mut builder = Paseto::<V4, Public>::builder();
        builder.set_payload(Payload::from(payload));
        builder.set_footer(Footer::from(footer.as_str()));
        builder.try_sign(&private).expect("sign")
    }

    // Claims whose `exp` is `exp_offset` seconds from a fixed reference
    // "now" comfortably in the future, so well-formed tokens are unexpired
    // regardless of the real clock; large negative offsets expire them.
    fn claims(exp_offset: i64) -> Claims {
        let now = 1_900_000_000; // year 2030
        Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            iss: ISSUER.to_string(),
            aud: AUDIENCE.to_string(),
            exp: now + exp_offset,
            iat: now,
            nbf: None,
            sid: "22222222-2222-2222-2222-222222222222".to_string(),
            scope: vec![],
            roles: vec![],
            attrs: BTreeMap::new(),
        }
    }

    #[test]
    fn valid_token_round_trips_claims() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        assert_eq!(verifier.key_count(), 1);
        let token = sign(KID, &claims(3600));
        let got = verifier.verify(&token).expect("verify");
        assert_eq!(got.sub, "11111111-1111-1111-1111-111111111111");
        assert_eq!(got.email, "alice@example.com");
        assert_eq!(got.iss, ISSUER);
        assert_eq!(got.aud, AUDIENCE);
        assert_eq!(got.sid, "22222222-2222-2222-2222-222222222222");
    }

    #[test]
    fn attrs_claim_round_trips_mint_to_verify() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let mut c = claims(3600);
        c.attrs.insert(
            "access".to_string(),
            vec!["write".to_string(), "admin".to_string()],
        );
        c.attrs
            .insert("dept".to_string(), vec!["cardiology".to_string()]);
        let token = sign(KID, &c);
        let got = verifier.verify(&token).expect("verify");
        assert_eq!(got.attrs, c.attrs);
    }

    #[test]
    fn absent_attrs_claim_verifies_to_empty_map() {
        // A pre-0.3 token carries no `attrs` member at all; it must verify
        // and land as an empty map — no re-issue needed.
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let now = 1_900_000_000_i64;
        let payload = serde_json::json!({
            "sub": "11111111-1111-1111-1111-111111111111",
            "email": "alice@example.com",
            "name": "Alice",
            "iss": ISSUER,
            "aud": AUDIENCE,
            "exp": now + 3600,
            "iat": now,
            "sid": "22222222-2222-2222-2222-222222222222",
        })
        .to_string();
        let token = sign_payload(KID, &payload);
        let got = verifier.verify(&token).expect("verify");
        assert!(got.attrs.is_empty());
    }

    #[test]
    fn expired_token_is_rejected() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let token = sign(KID, &claims(-10_000_000_000));
        assert!(matches!(
            verifier.verify(&token),
            Err(VerifyError::Claim(_))
        ));
    }

    #[test]
    fn not_yet_valid_token_is_rejected() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let mut c = claims(3600);
        c.nbf = Some(1_900_000_000); // year 2030, after the real clock
        let token = sign(KID, &c);
        assert!(matches!(
            verifier.verify(&token),
            Err(VerifyError::Claim(_))
        ));
    }

    #[test]
    fn wrong_audience_is_rejected() {
        let verifier =
            Verifier::from_paseto_keys_value(&test_keys(), ISSUER, "some-other-service").unwrap();
        let token = sign(KID, &claims(3600));
        assert!(matches!(
            verifier.verify(&token),
            Err(VerifyError::Claim(_))
        ));
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let verifier =
            Verifier::from_paseto_keys_value(&test_keys(), "some-other-issuer", AUDIENCE).unwrap();
        let token = sign(KID, &claims(3600));
        assert!(matches!(
            verifier.verify(&token),
            Err(VerifyError::Claim(_))
        ));
    }

    #[test]
    fn unknown_kid_is_rejected() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let token = sign("not-a-known-kid", &claims(3600));
        assert!(matches!(
            verifier.verify(&token),
            Err(VerifyError::UnknownKid(_))
        ));
    }

    #[test]
    fn tampered_payload_is_rejected() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let token = sign(KID, &claims(3600));
        // Flip a character in the payload segment (index 2 of v4.public.X.Y).
        let mut parts: Vec<&str> = token.split('.').collect();
        let mut payload = parts[2].to_string();
        let last = payload.len() - 1;
        let swapped = if &payload[last..] == "A" { "B" } else { "A" };
        payload.replace_range(last.., swapped);
        parts[2] = &payload;
        let tampered = parts.join(".");
        assert!(verifier.verify(&tampered).is_err());
    }

    #[test]
    fn garbage_token_is_rejected() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        assert!(verifier.verify("not.a.paseto").is_err());
        assert!(verifier.verify("").is_err());
        // A wrong-version token is rejected on the header check.
        assert!(matches!(
            verifier.verify("v2.public.aaaa"),
            Err(VerifyError::Malformed(_))
        ));
    }

    #[test]
    fn empty_key_set_builds_but_rejects_everything() {
        let keys = serde_json::json!({ "keys": [] });
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        assert_eq!(verifier.key_count(), 0);
        let token = sign(KID, &claims(3600));
        assert!(matches!(
            verifier.verify(&token),
            Err(VerifyError::UnknownKid(_))
        ));
    }

    #[test]
    fn key_set_without_keys_array_errors() {
        let keys = serde_json::json!({ "not_keys": [] });
        assert!(matches!(
            Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE),
            Err(VerifyError::Keys(_))
        ));
    }

    #[test]
    fn non_ed25519_keys_are_skipped() {
        let keys = serde_json::json!({
            "keys": [{ "kty": "RSA", "kid": "rsa-1", "n": "a", "e": "b" }]
        });
        let verifier = Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE).unwrap();
        assert_eq!(verifier.key_count(), 0);
    }

    #[test]
    fn ed25519_key_missing_kid_errors() {
        let public = signing_key().verifying_key().to_bytes();
        let x = URL_SAFE_NO_PAD.encode(public);
        let keys = serde_json::json!({
            "keys": [{ "kty": "OKP", "crv": "Ed25519", "x": x }]
        });
        assert!(matches!(
            Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE),
            Err(VerifyError::Keys(_))
        ));
    }

    #[test]
    fn ed25519_key_with_bad_x_errors() {
        let keys = serde_json::json!({
            "keys": [{ "kty": "OKP", "crv": "Ed25519", "kid": "bad-1", "x": "!!!not-base64!!!" }]
        });
        assert!(matches!(
            Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE),
            Err(VerifyError::Keys(_))
        ));
    }

    #[test]
    fn ed25519_key_with_wrong_length_x_errors() {
        // Valid base64url, but only 3 bytes — not a 32-byte Ed25519 key.
        let keys = serde_json::json!({
            "keys": [{ "kty": "OKP", "crv": "Ed25519", "kid": "short-1", "x": URL_SAFE_NO_PAD.encode([1, 2, 3]) }]
        });
        assert!(matches!(
            Verifier::from_paseto_keys_value(&keys, ISSUER, AUDIENCE),
            Err(VerifyError::Keys(_))
        ));
    }

    #[cfg(feature = "fetch")]
    #[tokio::test]
    async fn from_paseto_keys_url_maps_transport_error_to_fetch() {
        let result = Verifier::from_paseto_keys_url("not-a-url://nowhere", ISSUER, AUDIENCE).await;
        assert!(matches!(result, Err(VerifyError::Fetch(_))));
    }

    #[test]
    fn reloadable_verifier_swaps_the_key_set_for_rotation() {
        // Start with an empty key set (rejects every token), then
        // hot-swap to the real published key set — simulating a key
        // rotation picked up by a periodic re-fetch.
        let empty = serde_json::json!({ "keys": [] });
        let holder = ReloadableVerifier::new(
            Verifier::from_paseto_keys_value(&empty, ISSUER, AUDIENCE).unwrap(),
        );
        let token = sign(KID, &claims(3600));
        assert!(
            holder.current().verify(&token).is_err(),
            "before rotation: the empty key set rejects the token"
        );

        holder.store(Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap());
        assert!(
            holder.current().verify(&token).is_ok(),
            "after rotation: the token verifies against the new key set"
        );

        // A snapshot taken before a swap keeps the key set it captured.
        let snapshot = holder.current();
        holder.store(Verifier::from_paseto_keys_value(&empty, ISSUER, AUDIENCE).unwrap());
        assert!(
            snapshot.verify(&token).is_ok(),
            "an in-flight verification keeps the key set it snapshotted"
        );
        assert!(
            holder.current().verify(&token).is_err(),
            "new requests see the latest key set"
        );
    }

    // A DIFFERENT (attacker) seed → a keypair the published key set does NOT
    // contain. Signing with it while stamping the honest `kid` is the forgery
    // attempt the SEC-V4 test below must reject.
    const ATTACKER_SEED: [u8; 32] = [
        9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
        9, 9,
    ];

    fn attacker_sign_payload(kid: &str, payload: &str) -> String {
        let keypair = SigningKey::from_bytes(&ATTACKER_SEED).to_keypair_bytes();
        let key = Key::<64>::from(keypair);
        let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
        let footer = format!(r#"{{"kid":"{kid}"}}"#);
        let mut builder = Paseto::<V4, Public>::builder();
        builder.set_payload(Payload::from(payload));
        builder.set_footer(Footer::from(footer.as_str()));
        builder.try_sign(&private).expect("attacker sign")
    }

    /// SEC-V4 (the previously-missing forgery path): a token **validly signed
    /// by an attacker key** but stamped with the *honest* published `kid`
    /// must be rejected. The verifier selects the honest public key by `kid`,
    /// then the Ed25519 signature check fails — proving `kid` selection can't
    /// be abused to verify a token the honest key never signed.
    #[test]
    fn cross_key_forgery_with_honest_kid_is_rejected() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let payload = serde_json::to_string(&claims(3600)).unwrap();
        let forged = attacker_sign_payload(KID, &payload);
        assert!(matches!(
            verifier.verify(&forged),
            Err(VerifyError::Paseto(_))
        ));
    }

    /// SEC-V4: a token whose payload omits the required `exp` claim must be
    /// rejected (not treated as never-expiring) — `exp` is a non-`Option`
    /// field, so deserialization fails after the signature verifies.
    #[test]
    fn token_missing_exp_is_rejected() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let now = 1_900_000_000_i64;
        let payload = serde_json::json!({
            "sub": "11111111-1111-1111-1111-111111111111",
            "email": "alice@example.com",
            "name": "Alice",
            "iss": ISSUER,
            "aud": AUDIENCE,
            // no "exp"
            "iat": now,
            "sid": "22222222-2222-2222-2222-222222222222",
        })
        .to_string();
        let token = sign_payload(KID, &payload);
        assert!(verifier.verify(&token).is_err(), "missing exp must reject");
    }

    /// SEC-V4 (parser robustness / fuzz-lite): the verifier must only ever
    /// return `Err` — never panic — on arbitrary / malformed / truncated
    /// input. Pairs with `#![forbid(unsafe_code)]`.
    #[test]
    fn malformed_tokens_never_panic() {
        let verifier = Verifier::from_paseto_keys_value(&test_keys(), ISSUER, AUDIENCE).unwrap();
        let valid = sign(KID, &claims(3600));
        let mut cases: Vec<String> = vec![
            String::new(),
            ".".into(),
            "....".into(),
            "v4".into(),
            "v4.public".into(),
            "v4.public.".into(),
            "v4.public.!!!!".into(),
            "v4.local.deadbeef".into(),
            "v3.public.deadbeef".into(),
            "v4.public.YWJj.YWJj".into(),
            format!("v4.public.{}", "A".repeat(10_000)),
            format!("{valid}.extrasegment"),
            valid[..valid.len() / 2].to_string(),
            "🔥.🔥.🔥".into(),
        ];
        // A valid token with a wildly oversized footer.
        cases.push(sign_payload(
            &"k".repeat(5_000),
            &serde_json::to_string(&claims(3600)).unwrap(),
        ));
        for c in cases {
            // The contract: an `Err`, and above all no panic / no unwind.
            let _ = verifier.verify(&c);
        }
    }

    /// SEC-V1: the `fetch` path refuses a non-`https` key-set URL outright
    /// (before any network I/O), so a plaintext / downgraded fetch can't
    /// inject attacker keys. No network needed — the scheme check fails fast.
    #[cfg(feature = "fetch")]
    #[tokio::test]
    async fn non_https_keys_url_is_refused() {
        for url in [
            "http://auth.example.com/.well-known/paseto-keys",
            "ftp://x",
            "//x",
            "auth",
        ] {
            let r = Verifier::from_paseto_keys_url(url, ISSUER, AUDIENCE).await;
            assert!(
                matches!(r, Err(VerifyError::Fetch(_))),
                "non-https URL {url:?} must be refused"
            );
        }
    }
}
