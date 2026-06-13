//! Cross-crate sign→verify contract test (entity spec §13 T-4).
//!
//! The `Claims` struct and the `kid` derivation are duplicated **by
//! convention** between this service (`src/auth`) and the peer-side
//! [`authentication-verifier`] crate. Nothing but this test pins that
//! convention, so it exercises the full production contract:
//!
//! 1. load the RSA keypair the way the service does (`auth::keys()`,
//!    falling back to the committed dev keypair under `config/keys/`),
//! 2. sign a JWT via the service's `auth::sign_access_token`,
//! 3. take the JWKS document the service serves at
//!    `/.well-known/jwks.json` (`auth::keys().jwks`),
//! 4. build a `Verifier` from it and verify the token through the
//!    verifier crate,
//! 5. assert the claims round-trip and the `kid` contract holds.
//!
//! Pure crypto path — **no database required**, so this runs un-gated
//! in every `cargo test`.

use authentication_service::auth;
use authentication_verifier::{Verifier, VerifyError};
use base64::Engine;
use rsa::pkcs8::DecodePublicKey;
use rsa::traits::PublicKeyParts;
use sha2::{Digest, Sha256};

const PID: &str = "11111111-1111-1111-1111-111111111111";
const EMAIL: &str = "alice@example.com";
const NAME: &str = "Alice";

#[test]
fn service_signed_token_verifies_through_the_verifier_crate() {
    let keys = auth::keys();

    // The service signs (exactly as the magic-link redemption handler does).
    let (token, jti, exp) = auth::sign_access_token(PID, EMAIL, NAME).expect("sign");

    // A peer builds a Verifier from the *published* JWKS document with
    // the same issuer/audience policy the service stamps into tokens.
    let verifier =
        Verifier::from_jwks_value(&keys.jwks, &keys.issuer, &keys.audience).expect("build");
    assert_eq!(verifier.key_count(), 1, "the dev JWKS publishes one key");

    // The claims round-trip byte-for-byte through the duplicated struct.
    let claims = verifier.verify(&token).expect("peer-side verification");
    assert_eq!(claims.sub, PID);
    assert_eq!(claims.email, EMAIL);
    assert_eq!(claims.name, NAME);
    assert_eq!(claims.iss, keys.issuer);
    assert_eq!(claims.aud, keys.audience);
    assert_eq!(claims.jti, jti);
    assert_eq!(claims.exp, exp);
    assert!(claims.iat <= claims.exp);
}

#[test]
fn kid_contract_token_header_jwks_and_thumbprint_agree() {
    let keys = auth::keys();

    // 1. The kid stamped into the token header is the service's kid.
    let (token, _, _) = auth::sign_access_token(PID, EMAIL, NAME).expect("sign");
    let header = jsonwebtoken::decode_header(&token).expect("decode header");
    assert_eq!(header.kid.as_deref(), Some(keys.kid.as_str()));

    // 2. The published JWKS carries the same kid.
    let jwk = &keys.jwks["keys"][0];
    assert_eq!(jwk["kid"].as_str(), Some(keys.kid.as_str()));

    // 3. The kid is derived per the documented contract:
    //    base64url-no-pad( SHA-256( big-endian RSA modulus bytes ) ),
    //    recomputed here independently from the dev public key PEM.
    let public_pem = std::fs::read_to_string("config/keys/jwt_public_dev.pem")
        .expect("committed dev public key (auth::keys() loaded the same file)");
    let public_key = rsa::RsaPublicKey::from_public_key_pem(&public_pem).expect("parse");
    let n_bytes = public_key.n().to_bytes_be();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let expected_kid = b64.encode(Sha256::digest(&n_bytes));
    assert_eq!(keys.kid, expected_kid, "kid != base64url(sha256(modulus))");

    // …and the JWKS modulus is the same key, base64url-no-pad encoded.
    assert_eq!(jwk["n"].as_str(), Some(b64.encode(&n_bytes).as_str()));
}

#[test]
fn kid_mismatch_fails_peer_side_verification() {
    let keys = auth::keys();
    let (token, _, _) = auth::sign_access_token(PID, EMAIL, NAME).expect("sign");

    // A JWKS whose key is published under a different kid must not
    // verify the service's token: key selection is strictly by kid.
    let mut altered = keys.jwks.clone();
    altered["keys"][0]["kid"] = serde_json::Value::String("some-other-kid".to_string());
    let verifier =
        Verifier::from_jwks_value(&altered, &keys.issuer, &keys.audience).expect("build");
    assert!(
        matches!(verifier.verify(&token), Err(VerifyError::UnknownKid(kid)) if kid == keys.kid),
        "a kid mismatch must surface as UnknownKid(service kid)"
    );
}

#[test]
fn multi_key_jwks_verifies_primary_and_rejects_unknown_kid() {
    // Key rotation (T-5): a peer building a Verifier from a JWKS that
    // carries MORE than the primary key must still verify a primary-signed
    // token (selecting the primary by its kid) and must reject a token
    // signed under a kid absent from the published set.
    let keys = auth::keys();
    let (token, _, _) = auth::sign_access_token(PID, EMAIL, NAME).expect("sign");

    // Synthesise a second, unrelated RSA JWK and splice it into the
    // service's published JWKS — emulating a rotated-out additional key.
    let other_pub = "-----BEGIN PUBLIC KEY-----\n\
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAm/AOGzHpbO3hC0whTHF8\n\
iUzHUNJNpJQMZD4z0R3QLcM1furi4ygZSNjU7m5FJdAXCiyqV9RvZg43IkIoGuX6\n\
0P0F/SHnShnVN2i6NAb6XsACtk5kcA3uPIt3frhlj2w3A3fQxtIRM+roHQl3gnF3\n\
pHXLF/HvGxfROBWCvtg2iPcYlYTEEvMUmOzLdzrP0/BgU9u/yXSpM6F9mZ7WIjjo\n\
ccHMM7UvlvMtalQ+m9/YTwqNr25fp8hXTerc9NC/6mpCTFeACY1zmIU+TVwWCCPd\n\
r5pc52DVjU7QcppmhoXrO0WyG/QL4AtjyWXZOe0sZdLT+9QrCF9j/0YzwwKBMaTn\n\
wwIDAQAB\n\
-----END PUBLIC KEY-----\n";
    let pub_key = rsa::RsaPublicKey::from_public_key_pem(other_pub).expect("parse other pub");
    let n_bytes = pub_key.n().to_bytes_be();
    let e_bytes = pub_key.e().to_bytes_be();
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let other_kid = b64.encode(Sha256::digest(&n_bytes));

    let mut multi = keys.jwks.clone();
    multi["keys"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "kty": "RSA", "use": "sig", "alg": "RS256",
            "kid": other_kid, "n": b64.encode(&n_bytes), "e": b64.encode(&e_bytes),
        }));

    let verifier = Verifier::from_jwks_value(&multi, &keys.issuer, &keys.audience).expect("build");
    assert_eq!(
        verifier.key_count(),
        2,
        "the multi-key JWKS publishes two keys"
    );

    // The primary-signed token still verifies (kid selects the primary).
    let claims = verifier.verify(&token).expect("primary token verifies");
    assert_eq!(claims.sub, PID);

    // A JWKS that does NOT publish the signing kid must reject the token:
    // republish the primary's modulus under a different kid so no entry
    // matches the token header's kid.
    let header = jsonwebtoken::decode_header(&token).expect("header");
    let mut wrong = serde_json::json!({ "keys": [] });
    wrong["keys"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "kty": "RSA", "use": "sig", "alg": "RS256",
            "kid": "definitely-not-the-signing-kid",
            "n": keys.jwks["keys"][0]["n"], "e": keys.jwks["keys"][0]["e"],
        }));
    let no_match = Verifier::from_jwks_value(&wrong, &keys.issuer, &keys.audience).expect("build");
    assert!(
        matches!(no_match.verify(&token), Err(VerifyError::UnknownKid(kid)) if kid == header.kid.unwrap()),
        "a token whose kid is absent from the JWKS must be rejected as UnknownKid"
    );
}

#[test]
fn issuer_and_audience_policy_is_enforced_peer_side() {
    let keys = auth::keys();
    let (token, _, _) = auth::sign_access_token(PID, EMAIL, NAME).expect("sign");

    let wrong_audience =
        Verifier::from_jwks_value(&keys.jwks, &keys.issuer, "some-other-audience").expect("build");
    assert!(matches!(
        wrong_audience.verify(&token),
        Err(VerifyError::Jwt(_))
    ));

    let wrong_issuer =
        Verifier::from_jwks_value(&keys.jwks, "some-other-issuer", &keys.audience).expect("build");
    assert!(matches!(
        wrong_issuer.verify(&token),
        Err(VerifyError::Jwt(_))
    ));
}
