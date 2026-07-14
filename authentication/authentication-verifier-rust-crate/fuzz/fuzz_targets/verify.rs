//! SEC-I2 fuzz target: the offline PASETO `v4.public` token verifier.
//!
//! `Verifier::verify` runs on a **peer-supplied bearer token** — the most
//! directly attacker-controlled input the family exposes. This feeds it
//! arbitrary UTF-8 and pins the crate's golden rule #5: every failure mode
//! is a handled `VerifyError`, never a panic. The whole structural parse is
//! exercised — the `v4.public` header check, the authenticated footer's
//! base64url/JSON decode for its `kid`, key selection, and (for a token
//! that reaches it) the Ed25519 signature check — since the verifier is
//! built with a real key so the `kid`-found branch is reachable. A random
//! token cannot forge a signature, so a valid claim set is unreachable by
//! luck; the value here is the parser never aborting on hostile bytes.

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;
use authentication_verifier::Verifier;

fn verifier() -> &'static Verifier {
    static V: OnceLock<Verifier> = OnceLock::new();
    V.get_or_init(|| {
        // One Ed25519 JWK so the `kid`-lookup and signature-check branches
        // are reachable. `x` is a valid 32-byte base64url value (all zeros);
        // construction only decodes the bytes, and a signature check against
        // it fails gracefully — it never panics.
        let keys = serde_json::json!({
            "keys": [{
                "kty": "OKP",
                "crv": "Ed25519",
                "kid": "k1",
                "x": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
            }]
        });
        Verifier::from_paseto_keys_value(&keys, "authentication-service", "main-x-service")
            .expect("verifier builds from a valid Ed25519 key set")
    })
}

fuzz_target!(|data: &[u8]| {
    let Ok(token) = std::str::from_utf8(data) else {
        return;
    };
    // Any string is a handled `Ok`/`Err`, never a panic (golden rule #5).
    let _ = verifier().verify(token);
});
