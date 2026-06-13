//! Offline RS256 JWT verification against the authentication-service JWKS.
//!
//! The [`authentication-service`] is the federation's single auth
//! provider. It signs RS256 access tokens and publishes its public keys
//! at `/.well-known/jwks.json`. Every other service verifies those
//! tokens *offline*: fetch the JWKS once at boot, build a [`Verifier`],
//! then call [`Verifier::verify`] per request. There is no shared secret
//! and no per-request introspection call.
//!
//! This crate is the peer-side mirror of the auth-service's own
//! `auth::verify_token`: same RS256 algorithm, same [`Claims`] shape,
//! same `kid` selection — but keyed off the *published* JWKS rather than
//! a locally held private key, so any service can embed it.
//!
//! ```no_run
//! # use authentication_verifier::Verifier;
//! let jwks: serde_json::Value = serde_json::json!({ "keys": [] });
//! let verifier = Verifier::from_jwks_value(&jwks, "authentication-service", "main-x-service")?;
//! let claims = verifier.verify("eyJhbGci...")?;
//! println!("authenticated subject: {}", claims.sub);
//! # Ok::<(), authentication_verifier::VerifyError>(())
//! ```
//!
//! [`authentication-service`]: https://github.com/sixarm/authentication-service-rust-crate

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![deny(missing_docs)]

use std::collections::HashMap;

use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};

/// Verified token claims. Mirrors the auth-service `Claims` exactly so a
/// token signed there round-trips here. `sub` carries the user `pid`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user `pid` (UUID string).
    pub sub: String,
    /// User email, for convenience at the edge.
    pub email: String,
    /// Display name.
    pub name: String,
    /// Issuer (`iss`).
    pub iss: String,
    /// Audience (`aud`).
    pub aud: String,
    /// Expiry (unix seconds).
    pub exp: i64,
    /// Issued-at (unix seconds).
    pub iat: i64,
    /// JWT id — also the auth-service `sessions.jid`.
    pub jti: String,
}

/// Failure modes for JWKS loading and token verification.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// The JWKS document was missing or structurally invalid.
    #[error("malformed jwks: {0}")]
    Jwks(String),
    /// The token header carried no `kid`, so no key could be selected.
    #[error("token header has no kid")]
    MissingKid,
    /// No verification key matched the token's `kid`. The signer is
    /// unknown to this JWKS (stale cache, wrong issuer, or forgery).
    #[error("no verification key for kid {0:?}")]
    UnknownKid(String),
    /// Signature, issuer, audience, or expiry validation failed.
    #[error("token verification failed: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    /// Fetching the JWKS over HTTP failed (only with the `fetch` feature).
    #[cfg(feature = "fetch")]
    #[error("jwks fetch failed: {0}")]
    Fetch(String),
}

/// A set of RSA verification keys (indexed by `kid`) plus the issuer /
/// audience policy applied to every token. Construct once at boot, then
/// share behind an `Arc` and call [`verify`](Verifier::verify) per
/// request — verification is read-only and allocation-light.
pub struct Verifier {
    keys: HashMap<String, DecodingKey>,
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
    pub fn from_jwks_value(
        jwks: &serde_json::Value,
        issuer: &str,
        audience: &str,
    ) -> Result<Self, VerifyError> {
        let entries = jwks
            .get("keys")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| VerifyError::Jwks("missing \"keys\" array".to_string()))?;

        let mut keys = HashMap::new();
        for jwk in entries {
            let kty = jwk.get("kty").and_then(serde_json::Value::as_str);
            if kty != Some("RSA") {
                continue;
            }
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
            let key = DecodingKey::from_rsa_components(n, e)
                .map_err(|err| VerifyError::Jwks(format!("jwk {kid}: {err}")))?;
            keys.insert(kid.to_string(), key);
        }

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);
        Ok(Self { keys, validation })
    }

    /// Number of RSA verification keys loaded from the JWKS.
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.keys.len()
    }

    /// Verify an RS256 bearer token: select the key by the header `kid`,
    /// check the signature, then enforce issuer, audience, and expiry.
    ///
    /// # Errors
    ///
    /// [`VerifyError::MissingKid`] / [`VerifyError::UnknownKid`] when no
    /// key matches, or [`VerifyError::Jwt`] when signature/claim
    /// validation fails.
    pub fn verify(&self, token: &str) -> Result<Claims, VerifyError> {
        let header = decode_header(token)?;
        let kid = header.kid.ok_or(VerifyError::MissingKid)?;
        let key = self.keys.get(&kid).ok_or(VerifyError::UnknownKid(kid))?;
        let data = decode::<Claims>(token, key, &self.validation)?;
        Ok(data.claims)
    }
}

#[cfg(feature = "fetch")]
impl Verifier {
    /// Fetch the JWKS from `url` over HTTPS and build a verifier. Call
    /// once at boot; the auth-service rotates keys rarely, so a process
    /// can cache the result for its lifetime (or refetch on
    /// [`VerifyError::UnknownKid`] to pick up a rotation).
    ///
    /// # Errors
    ///
    /// [`VerifyError::Fetch`] on any transport / non-2xx / decode error,
    /// or [`VerifyError::Jwks`] when the fetched body is not a valid JWKS.
    pub async fn from_jwks_url(
        url: &str,
        issuer: &str,
        audience: &str,
    ) -> Result<Self, VerifyError> {
        let body = reqwest::get(url)
            .await
            .map_err(|e| VerifyError::Fetch(e.to_string()))?
            .error_for_status()
            .map_err(|e| VerifyError::Fetch(e.to_string()))?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| VerifyError::Fetch(e.to_string()))?;
        Self::from_jwks_value(&body, issuer, audience)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    const TEST_PUBLIC_PEM: &str = "-----BEGIN PUBLIC KEY-----\n\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAm/AOGzHpbO3hC0whTHF8\n\
iUzHUNJNpJQMZD4z0R3QLcM1furi4ygZSNjU7m5FJdAXCiyqV9RvZg43IkIoGuX6\n\
0P0F/SHnShnVN2i6NAb6XsACtk5kcA3uPIt3frhlj2w3A3fQxtIRM+roHQl3gnF3\n\
pHXLF/HvGxfROBWCvtg2iPcYlYTEEvMUmOzLdzrP0/BgU9u/yXSpM6F9mZ7WIjjo\n\
ccHMM7UvlvMtalQ+m9/YTwqNr25fp8hXTerc9NC/6mpCTFeACY1zmIU+TVwWCCPd\n\
r5pc52DVjU7QcppmhoXrO0WyG/QL4AtjyWXZOe0sZdLT+9QrCF9j/0YzwwKBMaTn\n\
wwIDAQAB\n\
-----END PUBLIC KEY-----\n";

    const ISSUER: &str = "authentication-service";
    const AUDIENCE: &str = "main-x-service";

    // Build a JWKS document from the test public key, mirroring exactly
    // how the auth-service derives (kid, n, e) in `auth::load_keys`.
    fn test_jwks() -> (serde_json::Value, String) {
        let pub_key = rsa::RsaPublicKey::from_public_key_pem(TEST_PUBLIC_PEM).expect("parse pub");
        let n_bytes = pub_key.n().to_bytes_be();
        let e_bytes = pub_key.e().to_bytes_be();
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let n = b64.encode(&n_bytes);
        let e = b64.encode(&e_bytes);
        let kid = b64.encode(Sha256::digest(&n_bytes));
        let jwks = serde_json::json!({
            "keys": [{
                "kty": "RSA", "use": "sig", "alg": "RS256",
                "kid": kid, "n": n, "e": e,
            }]
        });
        (jwks, kid)
    }

    fn sign(kid: &str, claims: &Claims) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(kid.to_string());
        let key = EncodingKey::from_rsa_pem(TEST_PRIVATE_PEM.as_bytes()).expect("encoding key");
        jsonwebtoken::encode(&header, claims, &key).expect("encode")
    }

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

    #[test]
    fn expired_token_is_rejected() {
        let (jwks, kid) = test_jwks();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        // exp far in the past relative to the body's "now".
        let token = sign(&kid, &claims(-10_000_000_000));
        assert!(matches!(verifier.verify(&token), Err(VerifyError::Jwt(_))));
    }

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

    #[test]
    fn garbage_token_is_rejected() {
        let (jwks, _) = test_jwks();
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        assert!(verifier.verify("not.a.jwt").is_err());
        assert!(verifier.verify("").is_err());
    }

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

    #[test]
    fn jwks_without_keys_array_errors() {
        let jwks = serde_json::json!({ "not_keys": [] });
        assert!(matches!(
            Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE),
            Err(VerifyError::Jwks(_))
        ));
    }

    #[test]
    fn non_rsa_keys_are_skipped() {
        let jwks = serde_json::json!({
            "keys": [{ "kty": "EC", "kid": "ec-1", "crv": "P-256", "x": "a", "y": "b" }]
        });
        let verifier = Verifier::from_jwks_value(&jwks, ISSUER, AUDIENCE).expect("build");
        assert_eq!(verifier.key_count(), 0);
    }
}
