//! Cross-crate sign→verify contract test (entity spec §13).
//!
//! The `Claims` struct and the footer-`kid` contract are duplicated **by
//! convention** between this service (`src/auth`) and the peer-side
//! [`authentication-verifier`] crate. Nothing but this test pins that
//! convention, so it exercises the full production contract:
//!
//! 1. load the Ed25519 signing key the way the service does
//!    (`auth::keys()`, falling back to the built-in dev seed),
//! 2. mint a PASETO `v4.public` token via `auth::sign_access_token`,
//! 3. take the key set the service serves at `/.well-known/paseto-keys`
//!    (`auth::keys().paseto_keys`),
//! 4. build a `Verifier` from it and verify the token through the
//!    verifier crate,
//! 5. assert the claims round-trip and the `kid` contract holds.
//!
//! Pure crypto path — **no database required**, so this runs un-gated in
//! every `cargo test`.

use authentication_service::auth;
use authentication_verifier::{Verifier, VerifyError};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

const PID: &str = "11111111-1111-1111-1111-111111111111";
const EMAIL: &str = "alice@example.com";
const NAME: &str = "Alice";
const SID: &str = "99999999-9999-9999-9999-999999999999";

/// Extract the `kid` from a `v4.public` token's (authenticated) footer.
fn footer_kid(token: &str) -> String {
    let seg = token.split('.').nth(3).expect("footer segment");
    let bytes = URL_SAFE_NO_PAD.decode(seg).expect("footer base64url");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("footer json");
    json["kid"].as_str().expect("footer kid").to_string()
}

/// Pins the end-to-end contract: a token signed by this service verifies
/// through the peer `Verifier` built from the published key set, and every
/// claim round-trips byte-for-byte across the duplicated `Claims` shape.
#[test]
fn service_signed_token_verifies_through_the_verifier_crate() {
    let keys = auth::keys();

    // The service mints (exactly as the magic-link redemption handler does).
    let (token, sid, exp) = auth::sign_access_token(PID, EMAIL, NAME, SID).expect("sign");
    assert_eq!(sid, SID);

    // A peer builds a Verifier from the *published* key set with the same
    // issuer/audience policy the service stamps into tokens.
    let verifier =
        Verifier::from_paseto_keys_value(&keys.paseto_keys, &keys.issuer, &keys.audience)
            .expect("build");
    assert_eq!(verifier.key_count(), 1, "the dev key set publishes one key");

    // The claims round-trip byte-for-byte through the duplicated struct.
    let claims = verifier.verify(&token).expect("peer-side verification");
    assert_eq!(claims.sub, PID);
    assert_eq!(claims.email, EMAIL);
    assert_eq!(claims.name, NAME);
    assert_eq!(claims.iss, keys.issuer);
    assert_eq!(claims.aud, keys.audience);
    assert_eq!(claims.sid, SID);
    assert_eq!(claims.exp, exp);
    assert!(claims.iat <= claims.exp);
}

/// Pins the `kid` derivation contract: the token-footer `kid`, the
/// published key-set `kid`, and an independently recomputed
/// `base64url(SHA-256(public key bytes))` all agree.
#[test]
fn kid_contract_token_footer_keyset_and_thumbprint_agree() {
    let keys = auth::keys();

    // 1. The kid stamped into the token footer is the service's kid.
    let (token, _, _) = auth::sign_access_token(PID, EMAIL, NAME, SID).expect("sign");
    assert_eq!(footer_kid(&token), keys.kid);

    // 2. The published key set carries the same kid.
    let jwk = &keys.paseto_keys["keys"][0];
    assert_eq!(jwk["kid"].as_str(), Some(keys.kid.as_str()));
    assert_eq!(jwk["kty"].as_str(), Some("OKP"));
    assert_eq!(jwk["crv"].as_str(), Some("Ed25519"));

    // 3. The kid is base64url-no-pad( SHA-256( raw public key bytes ) ),
    //    recomputed independently from the published `x`.
    let x = jwk["x"].as_str().expect("x");
    let public = URL_SAFE_NO_PAD.decode(x).expect("x base64url");
    let expected_kid = URL_SAFE_NO_PAD.encode(Sha256::digest(&public));
    assert_eq!(keys.kid, expected_kid, "kid != base64url(sha256(public))");
}

/// Pins strict key-selection-by-`kid`: republishing the key under a
/// different `kid` makes peer verification fail with `UnknownKid`.
#[test]
fn kid_mismatch_fails_peer_side_verification() {
    let keys = auth::keys();
    let (token, _, _) = auth::sign_access_token(PID, EMAIL, NAME, SID).expect("sign");

    let mut altered = keys.paseto_keys.clone();
    altered["keys"][0]["kid"] = serde_json::Value::String("some-other-kid".to_string());
    let verifier = Verifier::from_paseto_keys_value(&altered, &keys.issuer, &keys.audience)
        .expect("build");
    assert!(
        matches!(verifier.verify(&token), Err(VerifyError::UnknownKid(kid)) if kid == keys.kid),
        "a kid mismatch must surface as UnknownKid(service kid)"
    );
}

/// Pins the rotation contract on the peer side: a multi-key set still
/// verifies a primary-signed token (selecting the primary by `kid`) yet
/// rejects a token whose `kid` is absent from the set.
#[test]
fn multi_key_set_verifies_primary_and_rejects_unknown_kid() {
    let keys = auth::keys();
    let (token, _, _) = auth::sign_access_token(PID, EMAIL, NAME, SID).expect("sign");

    // Splice a second, unrelated Ed25519 public key (arbitrary 32 bytes)
    // under its own kid — emulating a rotated-out additional key.
    let other_public = [9u8; 32];
    let other_kid = URL_SAFE_NO_PAD.encode(Sha256::digest(other_public));
    let mut multi = keys.paseto_keys.clone();
    multi["keys"].as_array_mut().unwrap().push(serde_json::json!({
        "kty": "OKP", "crv": "Ed25519", "use": "sig",
        "kid": other_kid, "x": URL_SAFE_NO_PAD.encode(other_public),
    }));

    let verifier =
        Verifier::from_paseto_keys_value(&multi, &keys.issuer, &keys.audience).expect("build");
    assert_eq!(verifier.key_count(), 2, "the multi-key set publishes two keys");

    // The primary-signed token still verifies (kid selects the primary).
    let claims = verifier.verify(&token).expect("primary token verifies");
    assert_eq!(claims.sub, PID);

    // A key set that does NOT publish the signing kid rejects the token.
    let wrong = serde_json::json!({
        "keys": [{
            "kty": "OKP", "crv": "Ed25519", "use": "sig",
            "kid": "definitely-not-the-signing-kid",
            "x": keys.paseto_keys["keys"][0]["x"],
        }]
    });
    let no_match =
        Verifier::from_paseto_keys_value(&wrong, &keys.issuer, &keys.audience).expect("build");
    assert!(
        matches!(no_match.verify(&token), Err(VerifyError::UnknownKid(kid)) if kid == keys.kid),
        "a token whose kid is absent from the key set must be rejected as UnknownKid"
    );
}

/// Pins that `iss`/`aud` policy is enforced at the peer: a `Verifier`
/// built with the wrong audience or wrong issuer rejects an otherwise
/// valid token (signature alone is not sufficient).
#[test]
fn issuer_and_audience_policy_is_enforced_peer_side() {
    let keys = auth::keys();
    let (token, _, _) = auth::sign_access_token(PID, EMAIL, NAME, SID).expect("sign");

    let wrong_audience =
        Verifier::from_paseto_keys_value(&keys.paseto_keys, &keys.issuer, "some-other-audience")
            .expect("build");
    assert!(matches!(
        wrong_audience.verify(&token),
        Err(VerifyError::Claim(_))
    ));

    let wrong_issuer =
        Verifier::from_paseto_keys_value(&keys.paseto_keys, "some-other-issuer", &keys.audience)
            .expect("build");
    assert!(matches!(
        wrong_issuer.verify(&token),
        Err(VerifyError::Claim(_))
    ));
}
