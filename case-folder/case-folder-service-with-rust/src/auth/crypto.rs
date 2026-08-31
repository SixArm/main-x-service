//! A minimal, HS256-only `jsonwebtoken` [`CryptoProvider`].
//!
//! `jsonwebtoken` 10+ ships **no default crypto backend**
//! (`default = ["use_pem"]` only): `encode`/`decode` still compile
//! without one, but the installed default's signer/verifier factories
//! `panic!` at the first call — a runtime trap, not a compile error,
//! and one the fast (non-`--ignored`) test suite does not exercise,
//! because minting/verifying a magic-link token only happens inside
//! `src/controllers/auth.rs`'s DB-gated request tests. See the
//! round-trip test at the bottom of `src/auth/mod.rs`, which does
//! exercise it, for the regression guard.
//!
//! Of the two built-in backends, neither fits:
//! - `rust_crypto` bundles the `rsa` crate unconditionally (its
//!   `Cargo.toml` feature list has no finer split), reintroducing
//!   RUSTSEC-2023-0071 (the Marvin timing attack) — the exact crate
//!   this family removed from the dependency graph on 2026-08-21
//!   (`agents/share/security.md` §7) by dropping loco's `auth`
//!   feature. Enabling it here would undo that for a crate whose own
//!   `Cargo.toml` comment cites the same reasoning.
//! - `aws_lc_rs` avoids `rsa`, but pulls in a C/asm crypto library
//!   this family has no other dependency on.
//!
//! This crate signs and verifies exactly one algorithm — HS256, for
//! the short-lived magic-link token (`src/auth/mod.rs`; the session
//! itself is an opaque id, never a JWT, per `agents/share/jwt.md`) —
//! so a minimal custom provider, built from the same RustCrypto
//! `hmac`/`sha2` primitives `rust_crypto`'s own HS256 implementation
//! uses (see `jsonwebtoken`'s `src/crypto/rust_crypto/hmac.rs`),
//! covers the one algorithm this crate needs without either
//! trade-off. `CryptoProvider` is documented, public extension point
//! for exactly this ("provide your own custom implementation").

use hmac::{Hmac, KeyInit, Mac};
use jsonwebtoken::crypto::{CryptoProvider, JwtSigner, JwtVerifier, KeyUtils};
use jsonwebtoken::errors::{ErrorKind, Result as JwtResult, new_error};
use jsonwebtoken::signature::{self, Signer, Verifier};
use jsonwebtoken::{Algorithm, AlgorithmFamily, DecodingKey, EncodingKey};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Signs with HS256. Construction fails for any key that is not an
/// HMAC key or any algorithm that is not HS256 — mirroring
/// `jsonwebtoken`'s own HMAC signer.
struct Hs256Signer(HmacSha256);

impl Hs256Signer {
    fn new(key: &EncodingKey) -> JwtResult<Self> {
        if key.family() != AlgorithmFamily::Hmac {
            return Err(new_error(ErrorKind::InvalidKeyFormat));
        }
        let mac =
            HmacSha256::new_from_slice(key.as_bytes()).map_err(|_| ErrorKind::InvalidKeyFormat)?;
        Ok(Self(mac))
    }
}

impl Signer<Vec<u8>> for Hs256Signer {
    fn try_sign(&self, msg: &[u8]) -> Result<Vec<u8>, signature::Error> {
        let mut mac = self.0.clone();
        mac.update(msg);
        Ok(mac.finalize().into_bytes().to_vec())
    }
}

impl JwtSigner for Hs256Signer {
    fn algorithm(&self) -> Algorithm {
        Algorithm::HS256
    }
}

/// Verifies HS256 in constant time (`Mac::verify_slice`, not a `==`
/// comparison of the recomputed tag).
struct Hs256Verifier(HmacSha256);

impl Hs256Verifier {
    fn new(key: &DecodingKey) -> JwtResult<Self> {
        if key.family() != AlgorithmFamily::Hmac {
            return Err(new_error(ErrorKind::InvalidKeyFormat));
        }
        let mac = HmacSha256::new_from_slice(key.try_get_as_bytes()?)
            .map_err(|_| ErrorKind::InvalidKeyFormat)?;
        Ok(Self(mac))
    }
}

impl Verifier<Vec<u8>> for Hs256Verifier {
    fn verify(&self, msg: &[u8], signature: &Vec<u8>) -> Result<(), signature::Error> {
        let mut mac = self.0.clone();
        mac.update(msg);
        mac.verify_slice(signature)
            .map_err(signature::Error::from_source)
    }
}

impl JwtVerifier for Hs256Verifier {
    fn algorithm(&self) -> Algorithm {
        Algorithm::HS256
    }
}

/// Refuses every algorithm but HS256, cleanly (an `Err`, never a panic
/// or a silent fallback) — this service never signs or verifies
/// anything else.
///
/// `&Algorithm` (not `Algorithm`) is dictated by
/// [`CryptoProvider::signer_factory`]'s function-pointer type, not a
/// local style choice.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn new_signer(algorithm: &Algorithm, key: &EncodingKey) -> JwtResult<Box<dyn JwtSigner>> {
    match algorithm {
        Algorithm::HS256 => Ok(Box::new(Hs256Signer::new(key)?)),
        _ => Err(new_error(ErrorKind::InvalidAlgorithm)),
    }
}

/// The verifying half of [`new_signer`]'s restriction (same
/// externally-dictated signature).
#[allow(clippy::trivially_copy_pass_by_ref)]
fn new_verifier(algorithm: &Algorithm, key: &DecodingKey) -> JwtResult<Box<dyn JwtVerifier>> {
    match algorithm {
        Algorithm::HS256 => Ok(Box::new(Hs256Verifier::new(key)?)),
        _ => Err(new_error(ErrorKind::InvalidAlgorithm)),
    }
}

/// The process-wide provider. `key_utils` is `new_unimplemented`
/// (panics only if actually called) because this service never
/// touches JWKs — `Jwk`/`jwk` do not appear anywhere in this crate.
static PROVIDER: CryptoProvider = CryptoProvider {
    signer_factory: new_signer,
    verifier_factory: new_verifier,
    key_utils: KeyUtils::new_unimplemented(),
};

/// Install [`PROVIDER`] as the process default, if it is not already
/// installed.
///
/// `CryptoProvider::install_default` succeeds at most once per
/// process; every caller after the first gets an `Err`, which is
/// expected (every [`AuthState::new`](super::AuthState::new) call —
/// production boot and each test — calls this) and silently ignored
/// rather than surfaced, since "already installed" is not a failure
/// here.
pub(super) fn install() {
    let _ = PROVIDER.install_default();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Claims {
        sub: String,
    }

    #[test]
    fn hs256_signs_and_verifies_round_trip() {
        install();
        let encoding = EncodingKey::from_secret(b"a-test-secret");
        let decoding = DecodingKey::from_secret(b"a-test-secret");
        let header = jsonwebtoken::Header::new(Algorithm::HS256);
        let claims = Claims {
            sub: "alice@example.com".to_string(),
        };
        let token = jsonwebtoken::encode(&header, &claims, &encoding).expect("encode succeeds");
        let mut validation = jsonwebtoken::Validation::new(Algorithm::HS256);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        let data = jsonwebtoken::decode::<Claims>(&token, &decoding, &validation)
            .expect("decode succeeds");
        assert_eq!(data.claims, claims);
    }

    #[test]
    fn hs256_rejects_a_wrong_secret() {
        install();
        let encoding = EncodingKey::from_secret(b"right-secret");
        let wrong_decoding = DecodingKey::from_secret(b"wrong-secret");
        let header = jsonwebtoken::Header::new(Algorithm::HS256);
        let claims = Claims {
            sub: "alice@example.com".to_string(),
        };
        let token = jsonwebtoken::encode(&header, &claims, &encoding).expect("encode succeeds");
        let mut validation = jsonwebtoken::Validation::new(Algorithm::HS256);
        validation.required_spec_claims.clear();
        validation.validate_exp = false;
        assert!(jsonwebtoken::decode::<Claims>(&token, &wrong_decoding, &validation).is_err());
    }
}
