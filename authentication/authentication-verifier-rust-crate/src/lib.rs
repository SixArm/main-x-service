//! Offline RS256 JWT verification against the authentication-service JWKS.
//!
//! # The offline verification model
//!
//! The [`authentication-service`] is the federation's single auth
//! provider. It signs RS256 access tokens and publishes its public keys
//! at `/.well-known/jwks.json`. Every other service verifies those
//! tokens *offline*: fetch the JWKS once at boot, build a [`Verifier`],
//! then call [`Verifier::verify`] per request. There is no shared secret
//! and no per-request introspection call.
//!
//! "Offline" is the key property and the reason this crate exists.
//! Because RS256 is *asymmetric*, the signer (auth-service) holds the
//! private key and the verifiers (every peer service) hold only the
//! public key. A peer can therefore confirm a token's authenticity with
//! no network round-trip back to the auth-service on the hot path, no
//! shared symmetric secret to distribute and rotate, and no token
//! introspection endpoint to depend on for availability. The only
//! network interaction is the one-time (or rare) JWKS fetch, which can
//! be done out of band or via the optional [`fetch`](crate#features)
//! feature.
//!
//! # Security properties
//!
//! - **Asymmetric trust.** Verifiers never possess signing material, so
//!   a compromised peer cannot mint tokens — it can only verify them.
//! - **No shared secret.** Nothing symmetric is shared across the
//!   federation, so there is no single secret whose leak forges tokens
//!   everywhere.
//! - **Algorithm pinning.** Verification is hard-pinned to
//!   [`Algorithm::RS256`]; an attacker cannot downgrade to `none` or
//!   trick the verifier into treating the RSA public key as an HMAC
//!   secret (the classic "alg confusion" attack), because the
//!   [`Validation`] only accepts RS256.
//! - **Key selection by `kid`.** The verifying key is chosen by the
//!   token header's `kid`, which the auth-service derives
//!   deterministically from the key material itself (see [`Verifier`]),
//!   so a forged or stale `kid` simply fails to match any known key.
//! - **Issuer / audience / expiry enforcement.** Every token must also
//!   carry the expected `iss`, the expected `aud`, and an unexpired
//!   `exp`; a structurally valid signature alone is not sufficient.
//!
//! # This crate vs. the auth-service
//!
//! This crate is the peer-side mirror of the auth-service's own
//! `auth::verify_token`: same RS256 algorithm, same [`Claims`] shape,
//! same `kid` selection — but keyed off the *published* JWKS rather than
//! a locally held private key, so any service can embed it. The
//! [`Claims`] struct is duplicated by convention (not via a shared
//! type) and the service's contract test pins the round-trip, so the
//! two stay byte-compatible.
//!
//! # Features
//!
//! - `fetch` (off by default) — adds [`Verifier::from_jwks_url`], which
//!   pulls the JWKS over HTTPS via `reqwest` (rustls). With the feature
//!   off the crate does no I/O at all and the caller supplies the JWKS
//!   as a [`serde_json::Value`].
//!
//! # Example
//!
//! ```no_run
//! # use authentication_verifier::Verifier;
//! // Normally the JWKS comes from the auth-service; an empty set here
//! // keeps the doctest offline and dependency-free.
//! let jwks: serde_json::Value = serde_json::json!({ "keys": [] });
//! let verifier = Verifier::from_jwks_value(&jwks, "authentication-service", "main-x-service")?;
//! let claims = verifier.verify("eyJhbGci...")?;
//! println!("authenticated subject: {}", claims.sub);
//! # Ok::<(), authentication_verifier::VerifyError>(())
//! ```
//!
//! [`authentication-service`]: https://github.com/sixarm/authentication-service-rust-crate

// Reject `unsafe` outright: this crate touches only safe, allocation-light
// code paths and a security library has no business reaching for `unsafe`.
#![forbid(unsafe_code)]
// Opt into clippy's pedantic lints to keep the public-facing library tidy.
#![warn(clippy::pedantic)]
// A published library must document every public item; fail the build if
// any item lacks a doc comment.
#![deny(missing_docs)]

// `HashMap` indexes the loaded RSA keys by their `kid` for O(1) lookup
// during verification.
use std::collections::HashMap;

// `jsonwebtoken` supplies the actual RS256 decode/verify primitives and the
// header parser used to read the `kid` before a key is selected.
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
// `serde` derives let `Claims` deserialize from the JWT payload (and
// serialize again, which the tests rely on to sign tokens).
use serde::{Deserialize, Serialize};

/// Verified token claims. Mirrors the auth-service `Claims` exactly so a
/// token signed there round-trips here. `sub` carries the user `pid`.
///
/// The field set and order are a contract with the auth-service: the
/// service defines an identical struct, and changing one without the
/// other breaks token round-tripping. `Deserialize` is what
/// [`Verifier::verify`] uses to reconstruct this struct from a verified
/// JWT payload; `Serialize` exists so callers (and the crate's own
/// tests) can re-encode claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user `pid` (UUID string). This is the stable
    /// identifier a peer service keys its own authorization on.
    pub sub: String,
    /// User email, surfaced for convenience at the edge so callers need
    /// not look it up separately; not used for authorization decisions.
    pub email: String,
    /// Human-readable display name carried alongside the subject.
    pub name: String,
    /// Issuer (`iss`) — the name of the auth-service that minted the
    /// token. Checked against the verifier's configured issuer.
    pub iss: String,
    /// Audience (`aud`) — the intended recipient service. Checked
    /// against the verifier's configured audience so a token issued for
    /// one peer cannot be replayed against another.
    pub aud: String,
    /// Expiry (`exp`), unix seconds. Tokens past this instant are
    /// rejected during verification.
    pub exp: i64,
    /// Issued-at (`iat`), unix seconds — when the token was minted.
    pub iat: i64,
    /// JWT id (`jti`) — also the auth-service `sessions.jid`, so a token
    /// can be correlated back to its server-side session.
    pub jti: String,
}

/// Failure modes for JWKS loading and token verification.
///
/// Every fallible entry point in this crate returns this type, and every
/// variant is a *handled* outcome — the crate never panics on bad input,
/// so an empty or malformed JWKS and a forged token are both ordinary
/// `Err` values. Variants split into load-time failures ([`Jwks`](Self::Jwks),
/// [`Fetch`](Self::Fetch)) and per-token verification failures
/// ([`MissingKid`](Self::MissingKid), [`UnknownKid`](Self::UnknownKid),
/// [`Jwt`](Self::Jwt)).
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// The JWKS document was missing or structurally invalid (no `keys`
    /// array, a key missing `kid` / `n` / `e`, or unparsable RSA
    /// components). Raised at load time, not per token.
    #[error("malformed jwks: {0}")]
    Jwks(String),
    /// The token header carried no `kid`, so no key could be selected.
    /// All auth-service tokens stamp a `kid`, so this indicates a
    /// hand-built or non-conforming token.
    #[error("token header has no kid")]
    MissingKid,
    /// No verification key matched the token's `kid`. The signer is
    /// unknown to this JWKS (stale cache, wrong issuer, or forgery). The
    /// wrapped `String` is the unmatched `kid`. On a legitimate stale
    /// cache, a caller may refetch the JWKS and retry.
    #[error("no verification key for kid {0:?}")]
    UnknownKid(String),
    /// Signature, issuer, audience, or expiry validation failed. Wraps
    /// the underlying `jsonwebtoken` error, which distinguishes the
    /// specific cause (bad signature vs. wrong `iss` / `aud` vs. expired
    /// `exp`). This is the catch-all for a key matched but the token
    /// itself did not validate.
    #[error("token verification failed: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    /// Fetching the JWKS over HTTP failed (only with the `fetch` feature):
    /// transport error, non-2xx status, or undecodable body. The wrapped
    /// `String` is the stringified transport error.
    #[cfg(feature = "fetch")]
    #[error("jwks fetch failed: {0}")]
    Fetch(String),
}

/// A set of RSA verification keys (indexed by `kid`) plus the issuer /
/// audience policy applied to every token. Construct once at boot, then
/// share behind an `Arc` and call [`verify`](Verifier::verify) per
/// request — verification is read-only and allocation-light.
///
/// The `kid` derivation is a contract with the auth-service: `kid` is
/// the base64url (no padding) encoding of `SHA-256(big-endian RSA
/// modulus bytes)`, which the service stamps into every token header.
/// Because the verifier indexes its keys by exactly that `kid`, key
/// selection at verify time is a direct map lookup with no ambiguity.
pub struct Verifier {
    /// RSA verifying keys keyed by their `kid`. Populated at construction
    /// from the JWKS; never mutated afterwards, which is why a `Verifier`
    /// is safe to share immutably across threads.
    keys: HashMap<String, DecodingKey>,
    /// The shared validation policy applied to every token: algorithm
    /// pinned to RS256, plus the expected issuer and audience. `exp`
    /// checking is on by `jsonwebtoken`'s default.
    validation: Validation,
}

impl Verifier {
    /// Build a verifier from an in-memory JWKS document, validating
    /// tokens against `issuer` (`iss`) and `audience` (`aud`).
    ///
    /// Only RSA signing keys are loaded; entries with a non-`RSA` `kty`
    /// are skipped (the auth-service publishes RS256 only). An empty key
    /// set is permitted — it yields a verifier that rejects every token
    /// with [`VerifyError::UnknownKid`], which lets a service boot before
    /// its JWKS source is reachable without panicking.
    ///
    /// # Errors
    ///
    /// [`VerifyError::Jwks`] when the document lacks a `keys` array, a key
    /// is missing `kid` / `n` / `e`, or the modulus/exponent are not
    /// valid RSA components.
    ///
    /// # Examples
    ///
    /// ```
    /// # use authentication_verifier::Verifier;
    /// // An empty key set still builds a usable verifier (it just
    /// // rejects every token with `UnknownKid`).
    /// let jwks = serde_json::json!({ "keys": [] });
    /// let verifier = Verifier::from_jwks_value(&jwks, "authentication-service", "main-x-service")?;
    /// assert_eq!(verifier.key_count(), 0);
    /// # Ok::<(), authentication_verifier::VerifyError>(())
    /// ```
    pub fn from_jwks_value(
        jwks: &serde_json::Value,
        issuer: &str,
        audience: &str,
    ) -> Result<Self, VerifyError> {
        // A JWKS is `{ "keys": [ ... ] }`; the top-level `keys` array is
        // mandatory. Its absence is a structural error, not an empty set.
        let entries = jwks
            .get("keys")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| VerifyError::Jwks("missing \"keys\" array".to_string()))?;

        let mut keys = HashMap::new();
        for jwk in entries {
            // RS256 only: silently skip any non-RSA key (e.g. EC) rather
            // than erroring, since a JWKS may legitimately advertise other
            // key types this verifier does not use.
            let kty = jwk.get("kty").and_then(serde_json::Value::as_str);
            if kty != Some("RSA") {
                continue;
            }
            // For an RSA key the three fields below are all required: the
            // `kid` is the lookup key, and `n` (modulus) + `e` (exponent)
            // are the public key components. Any missing field is a
            // malformed JWKS rather than a skippable entry.
            let kid = jwk
                .get("kid")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| VerifyError::Jwks("rsa jwk missing \"kid\"".to_string()))?;
            let n = jwk
                .get("n")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| VerifyError::Jwks(format!("jwk {kid} missing \"n\"")))?;
            let e = jwk
                .get("e")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| VerifyError::Jwks(format!("jwk {kid} missing \"e\"")))?;
            // Build the decoding key from the base64url modulus/exponent.
            // A parse failure here means the components are not valid RSA.
            let key = DecodingKey::from_rsa_components(n, e)
                .map_err(|err| VerifyError::Jwks(format!("jwk {kid}: {err}")))?;
            // Index by `kid`; a later key with the same `kid` overwrites.
            keys.insert(kid.to_string(), key);
        }

        // Pin the validation policy once. Hard-coding RS256 here is the
        // algorithm-confusion defense: a token claiming any other `alg`
        // (including `none`) is rejected before signature checking.
        let mut validation = Validation::new(Algorithm::RS256);
        // Require the configured issuer and audience on every token; these
        // are enforced by `decode` alongside the signature and expiry.
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);
        Ok(Self { keys, validation })
    }

    /// Number of RSA verification keys loaded from the JWKS.
    ///
    /// Useful as a boot-time sanity check: a count of zero means no token
    /// can ever verify (every [`verify`](Self::verify) call will return
    /// [`VerifyError::UnknownKid`]), which usually signals a JWKS that
    /// failed to load or advertised no RSA keys.
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Verify an RS256 bearer token: select the key by the header `kid`,
    /// check the signature, then enforce issuer, audience, and expiry.
    ///
    /// The steps run in a deliberate order so the cheapest, most specific
    /// rejection happens first: parse the header (no crypto), select the
    /// key by `kid`, and only then perform the RSA signature check and
    /// claim validation.
    ///
    /// # Errors
    ///
    /// - [`VerifyError::Jwt`] if the token header itself is unparsable
    ///   (the leading [`decode_header`] step).
    /// - [`VerifyError::MissingKid`] if the header carries no `kid`.
    /// - [`VerifyError::UnknownKid`] if the `kid` matches none of the
    ///   loaded keys (stale JWKS cache, wrong issuer, or forgery).
    /// - [`VerifyError::Jwt`] if the matched key's signature check fails,
    ///   or the `iss` / `aud` / `exp` claims do not satisfy the policy.
    ///
    /// # Examples
    ///
    /// ```
    /// # use authentication_verifier::{Verifier, VerifyError};
    /// // With no keys loaded, verification fails fast on `kid` lookup.
    /// let jwks = serde_json::json!({ "keys": [] });
    /// let verifier = Verifier::from_jwks_value(&jwks, "authentication-service", "main-x-service")?;
    /// assert!(verifier.verify("not.a.jwt").is_err());
    /// # Ok::<(), VerifyError>(())
    /// ```
    pub fn verify(&self, token: &str) -> Result<Claims, VerifyError> {
        // 1. Parse the header without verifying anything — this is just
        //    base64 decoding of the first JWT segment, so it is safe to do
        //    before a key is chosen and cannot be trusted yet.
        let header = decode_header(token)?;
        // 2. The `kid` tells us which published key signed this token. A
        //    missing `kid` is a distinct, more actionable error than an
        //    unknown one.
        let kid = header.kid.ok_or(VerifyError::MissingKid)?;
        // 3. Look up the key. A miss means we hold no key for this signer:
        //    stale cache, wrong issuer, or an outright forged `kid`.
        let key = self.keys.get(&kid).ok_or(VerifyError::UnknownKid(kid))?;
        // 4. Now do the real work: RSA signature verification plus the
        //    issuer/audience/expiry checks baked into `self.validation`.
        //    Only a token that passes all of these yields claims.
        let data = decode::<Claims>(token, key, &self.validation)?;
        Ok(data.claims)
    }
}

/// HTTP-loading constructor, available only with the `fetch` feature.
///
/// Kept in its own `cfg`-gated `impl` so the default build pulls in no
/// HTTP stack and does no I/O whatsoever.
#[cfg(feature = "fetch")]
impl Verifier {
    /// Fetch the JWKS from `url` over HTTPS and build a verifier. Call
    /// once at boot; the auth-service rotates keys rarely, so a process
    /// can cache the result for its lifetime (or refetch on
    /// [`VerifyError::UnknownKid`] to pick up a rotation).
    ///
    /// This is a convenience over [`from_jwks_value`](Self::from_jwks_value):
    /// it performs the network fetch and then delegates parsing and key
    /// loading to that method, so the validation policy is identical.
    ///
    /// # Errors
    ///
    /// [`VerifyError::Fetch`] on any transport / non-2xx / decode error,
    /// or [`VerifyError::Jwks`] when the fetched body is not a valid JWKS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use authentication_verifier::Verifier;
    /// # async fn run() -> Result<(), authentication_verifier::VerifyError> {
    /// let verifier = Verifier::from_jwks_url(
    ///     "https://auth.example.com/.well-known/jwks.json",
    ///     "authentication-service",
    ///     "main-x-service",
    /// )
    /// .await?;
    /// # let _ = verifier;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_jwks_url(
        url: &str,
        issuer: &str,
        audience: &str,
    ) -> Result<Self, VerifyError> {
        // Each stage maps its transport-layer error into `Fetch` so the
        // caller sees one uniform "could not obtain the JWKS" variant:
        let body = reqwest::get(url)
            .await
            // ...the GET itself (DNS, TLS, connection) failed,
            .map_err(|e| VerifyError::Fetch(e.to_string()))?
            // ...the server answered with a non-2xx status,
            .error_for_status()
            .map_err(|e| VerifyError::Fetch(e.to_string()))?
            // ...or the body was not decodable JSON.
            .json::<serde_json::Value>()
            .await
            .map_err(|e| VerifyError::Fetch(e.to_string()))?;
        // Reuse the in-memory constructor so structural validation and the
        // RS256/issuer/audience policy stay in exactly one place.
        Self::from_jwks_value(&body, issuer, audience)
    }
}

/// Offline unit tests.
///
/// The whole suite runs without network access: a throwaway RSA keypair
/// (below) plays the auth-service's signing role, a JWKS is derived from
/// its public half exactly as the service would, and tokens are signed
/// locally so each verification path can be exercised deterministically.
#[cfg(test)]
mod tests {
    use super::*;
    // `Engine` brings the base64 `encode` method into scope for the JWKS
    // builder. `EncodingKey`/`Header` let the tests sign tokens (the
    // signing side the library itself never performs). The `rsa`/`sha2`
    // imports reproduce the auth-service's `kid` derivation.
    use base64::Engine;
    use jsonwebtoken::{EncodingKey, Header};
    use rsa::pkcs8::DecodePublicKey;
    use rsa::traits::PublicKeyParts;
    use sha2::{Digest, Sha256};

    // Throwaway 2048-bit RSA keypair, used only to exercise the
    // verifier offline (sign here with the private key, verify against a
    // JWKS built from the public key). Not used anywhere in production.
    const TEST_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
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

    // The public half of `TEST_PRIVATE_PEM`. The JWKS the verifier loads
    // is derived from this key, so tokens signed with the private half
    // verify and tokens signed with any other key do not.
    const TEST_PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----\n\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAm/AOGzHpbO3hC0whTHF8\n\
iUzHUNJNpJQMZD4z0R3QLcM1furi4ygZSNjU7m5FJdAXCiyqV9RvZg43IkIoGuX6\n\
0P0F/SHnShnVN2i6NAb6XsACtk5kcA3uPIt3frhlj2w3A3fQxtIRM+roHQl3gnF3\n\
pHXLF/HvGxfROBWCvtg2iPcYlYTEEvMUmOzLdzrP0/BgU9u/yXSpM6F9mZ7WIjjo\n\
ccHMM7UvlvMtalQ+m9/YTwqNr25fp8hXTerc9NC/6mpCTFeACY1zmIU+TVwWCCPd\n\
r5pc52DVjU7QcppmhoXrO0WyG/QL4AtjyWXZOe0sZdLT+9QrCF9j/0YzwwKBMaTn\n\
wwIDAQAB\n\
-----END PUBLIC KEY-----\n";

    // The issuer/audience the verifier is configured with, and the values
    // stamped into well-formed test tokens. Tests that pass a *different*
    // issuer/audience to the verifier exercise the policy-mismatch paths.
    const ISSUER: &str = "authentication-service";
    const AUDIENCE: &str = "main-x-service";

    // Build a JWKS document from the test public key, mirroring exactly
    // how the auth-service derives (kid, n, e) in `auth::load_keys`.
    // Returns the JWKS plus its single `kid` so tokens can be signed to
    // match. The `kid` is base64url(SHA-256(modulus-bytes)) — the contract
    // the library relies on for key selection.
    fn test_jwks() -> (serde_json::Value, String) {
        let pub_key = rsa::RsaPublicKey::from_public_key_pem(TEST_PUBLIC_PEM).expect("parse pub");
        // Big-endian modulus and exponent are the raw RSA components a JWK
        // carries as base64url `n`/`e`.
        let n_bytes = pub_key.n().to_bytes_be();
        let e_bytes = pub_key.e().to_bytes_be();
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let n = b64.encode(&n_bytes);
        let e = b64.encode(&e_bytes);
        // `kid` derived from the modulus bytes — must match what the
        // verifier expects so key lookup succeeds.
        let kid = b64.encode(Sha256::digest(&n_bytes));
        let jwks = serde_json::json!({
            "keys": [{
                "kty": "RSA", "use": "sig", "alg": "RS256",
                "kid": kid, "n": n, "e": e,
            }]
        });
        (jwks, kid)
    }

    // Sign `claims` into an RS256 JWT with the given `kid` in the header,
    // using the test private key — the inverse of what the verifier does.
    fn sign(kid: &str, claims: &Claims) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_PEM.as_bytes()).expect("encoding key");
        jsonwebtoken::encode(&header, claims, &key).expect("encode")
    }

    // Construct a `Claims` whose `exp` is `exp_offset` seconds from a
    // fixed reference "now". Positive offsets are valid; large negative
    // offsets produce expired tokens.
    fn claims(exp_offset: i64) -> Claims {
        // Fixed iat well in the past; exp relative to a fixed "now" so the
        // test stays deterministic without reading the clock for the body.
        let now = 1_900_000_000; // year 2030, comfortably non-expired
        Claims {
            sub: "11111111-1111-1111-1111-111111111111".to_string(),
            email: "alice@example.com".to_string(),
            name: "Alice".to_string(),
            iss: ISSUER.to_string(),
            aud: AUDIENCE.to_string(),
            exp: now + exp_offset,
            iat: now,
            jti: "22222222-2222-2222-2222-222222222222".to_string(),
        }
    }

    // Happy path: a token signed by the matching key with the expected
    // issuer/audience/expiry verifies, and every claim field survives the
    // round-trip intact. This pins the `Claims` shape contract.
    #[test]
    fn valid_token_round_trips_claims() {
        let (jwks, kid) = test_jwks();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        assert_eq!(verifier.key_count(), 1);

        let token = sign(&kid, &claims(3600));
        let got = verifier.verify(&token).expect("verify");
        assert_eq!(got.sub, "11111111-1111-1111-1111-111111111111");
        assert_eq!(got.email, "alice@example.com");
        assert_eq!(got.iss, ISSUER);
        assert_eq!(got.aud, AUDIENCE);
    }

    // Pins expiry enforcement: a valid signature with an `exp` in the
    // past is still rejected, surfacing as a `Jwt` validation error.
    #[test]
    fn expired_token_is_rejected() {
        let (jwks, kid) = test_jwks();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        // exp far in the past relative to the body's "now".
        let token = sign(&kid, &claims(-10_000_000_000));
        assert!(matches!(verifier.verify(&token), Err(VerifyError::Jwt(_))));
    }

    // Pins audience enforcement: a fully valid token is rejected when the
    // verifier's configured audience differs from the token's `aud`,
    // preventing cross-service token replay.
    #[test]
    fn wrong_audience_is_rejected() {
        let (jwks, kid) = test_jwks();
        let verifier =
            Verifier::from_jwks_value(&jwks, ISSUER, "some-other-service").expect("build");
        let token = sign(&kid, &claims(3600));
        assert!(matches!(verifier.verify(&token), Err(VerifyError::Jwt(_))));
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        // The verifier's policy demands `iss == ISSUER`. A token whose
        // `iss` claim names a different issuer must be rejected even
        // though the signature, kid, audience, and expiry are all valid.
        let (jwks, kid) = test_jwks();
        let verifier =
            Verifier::from_jwks_value(&jwks, "some-other-issuer", AUDIENCE).expect("build");
        let token = sign(&kid, &claims(3600));
        assert!(matches!(verifier.verify(&token), Err(VerifyError::Jwt(_))));
    }

    // Pins `kid` selection: a token whose header `kid` is not in the JWKS
    // is rejected with the specific `UnknownKid` variant before any
    // signature work. The trailing `let _ = kid;` discards the real `kid`,
    // which this test deliberately does not use.
    #[test]
    fn unknown_kid_is_rejected() {
        let (jwks, kid) = test_jwks();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        // Sign with a kid the JWKS does not contain.
        let token = sign("not-a-known-kid", &claims(3600));
        assert!(matches!(
            verifier.verify(&token),
            Err(VerifyError::UnknownKid(_))
        ));
        let _ = kid;
    }

    // Pins signature integrity: corrupting one byte of an otherwise valid
    // token makes the RSA signature check fail, so verification errors.
    #[test]
    fn tampered_signature_is_rejected() {
        let (jwks, kid) = test_jwks();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        let token = sign(&kid, &claims(3600));
        let mut bytes = token.into_bytes();
        let last = bytes.len() - 1;
        // Flip a bit in the final signature char (avoid producing the same char).
        bytes[last] = if bytes[last] == b'A' { b'B' } else { b'A' };
        let tampered = String::from_utf8(bytes).unwrap();
        assert!(verifier.verify(&tampered).is_err());
    }

    // Pins robustness against non-JWT input: a malformed string and the
    // empty string both error rather than panic.
    #[test]
    fn garbage_token_is_rejected() {
        let (jwks, _) = test_jwks();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        assert!(verifier.verify("not.a.jwt").is_err());
        assert!(verifier.verify("").is_err());
    }

    // Pins the "boot before JWKS is reachable" guarantee: an empty key set
    // constructs successfully, reports zero keys, and rejects any token
    // with `UnknownKid` (never a panic).
    #[test]
    fn empty_jwks_builds_but_rejects_everything() {
        let jwks = serde_json::json!({ "keys": [] });
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        assert_eq!(verifier.key_count(), 0);
        let (_, kid) = test_jwks();
        let token = sign(&kid, &claims(3600));
        assert!(matches!(
            verifier.verify(&token),
            Err(VerifyError::UnknownKid(_))
        ));
    }

    // Pins the structural-validation boundary: a document with no `keys`
    // array is a load-time `Jwks` error, distinct from an empty key set.
    #[test]
    fn jwks_without_keys_array_errors() {
        let jwks = serde_json::json!({ "not_keys": [] });
        assert!(matches!(
            Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE),
            Err(VerifyError::Jwks(_))
        ));
    }

    // Pins the RS256-only policy: a non-RSA (here EC) key is silently
    // skipped during loading rather than erroring, leaving zero usable
    // keys.
    #[test]
    fn non_rsa_keys_are_skipped() {
        let jwks = serde_json::json!({
            "keys": [{ "kty": "EC", "kid": "ec-1", "crv": "P-256", "x": "a", "y": "b" }]
        });
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        assert_eq!(verifier.key_count(), 0);
    }
}
