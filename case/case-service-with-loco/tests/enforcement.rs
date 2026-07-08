//! End-to-end **blanket enforcement + ABAC** proof over the real HTTP
//! stack with `CASE_REQUIRE_AUTH` **on** (spec §13; the "activation
//! proof"). The DB-free unit tests in `src/auth.rs` pin the pure
//! `enforce`/`evaluate` matrix; this pins that the middleware is actually
//! wired on the booted router and that a real request with (or without) a
//! valid PASETO gets the right status against a real database.
//!
//! It runs in its **own test binary** so the process-wide `require_auth`
//! / `policy` / `verifier` `OnceLock`s are isolated from the
//! enforcement-**off** request suite in `tests/requests/`.
//!
//! `#[ignore]`d: it boots the app (needs PostgreSQL via `config/test.yaml`
//! / `DATABASE_URL`). Run with `cargo test --test enforcement -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use case_service::app::App;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

/// Throwaway Ed25519 seed — mints test tokens + a matching published key
/// set in-process. Not a secret; never used in production.
const SEED: [u8; 32] = [9; 32];
/// Issuer / audience the case service verifies against by default.
const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";

/// The published key set (as the auth service would serve it) and the
/// `kid` selecting the throwaway key.
fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// Mint a signed PASETO `v4.public` bearer for the test identity with the
/// given ABAC `attrs`, valid far into the future.
fn mint(kid: &str, attrs: &[(&str, &[&str])]) -> String {
    let attrs_map: serde_json::Map<String, Value> = attrs
        .iter()
        .map(|(k, vs)| {
            (
                (*k).to_string(),
                Value::Array(vs.iter().map(|v| Value::String((*v).to_string())).collect()),
            )
        })
        .collect();
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": "11111111-1111-1111-1111-111111111111",
        "email": "alice@example.com", "name": "Alice",
        "iss": ISSUER, "aud": AUDIENCE,
        "exp": iat + 10_000_000_000_i64, "iat": iat,
        "sid": "test-sid", "attrs": attrs_map,
    })
    .to_string();

    let keypair = SigningKey::from_bytes(&SEED).to_keypair_bytes();
    let key = Key::<64>::from(keypair);
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let footer = format!(r#"{{"kid":"{kid}"}}"#);
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(Payload::from(payload.as_str()));
    builder.set_footer(Footer::from(footer.as_str()));
    builder.try_sign(&private).expect("sign")
}

/// A minimal valid case body (the DTO is `case_matcher::Case`).
fn housing_case() -> Value {
    json!({ "title": "Housing benefit appeal", "agency_id": "dwp" })
}

/// Turn a bearer token into the `Authorization` header pair.
fn auth_header(token: &str) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    (
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    )
}

/// The full activation matrix in one test (one boot ⇒ one set of cached
/// `OnceLock`s): with `CASE_REQUIRE_AUTH=1` and the built-in default
/// policy, a public path is open, a protected path needs a token, and the
/// derived action is checked against the token's `attrs`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_on_gates_the_real_stack() {
    let (keys, kid) = keys_and_kid();
    // Enable enforcement + the throwaway key set BEFORE the app boots (the
    // auth OnceLocks read these on first use). `set_var` is `unsafe` in
    // edition 2024; this is a single-threaded test setup step.
    unsafe {
        std::env::set_var("CASE_REQUIRE_AUTH", "1");
        std::env::set_var("CASE_PASETO_KEYS", keys.to_string());
        // No CASE_ABAC_POLICY ⇒ the built-in default policy (read allow,
        // mutation deny, access=write ⇒ write).
    }

    request::<App, _, _>(|request, _ctx| async move {
        // Public path: open even with enforcement on, no token.
        assert_eq!(
            request.get("/_health").await.status_code(),
            200,
            "public health path stays open"
        );

        // Protected paths without a token ⇒ 401.
        assert_eq!(
            request.get("/api/cases").await.status_code(),
            401,
            "protected read without a token is 401"
        );
        assert_eq!(
            request
                .post("/api/cases")
                .json(&housing_case())
                .await
                .status_code(),
            401,
            "protected write without a token is 401"
        );

        // Valid token, empty attrs ⇒ read allowed (default allow-read),
        // write denied (default deny-mutation).
        let (rk, rv) = auth_header(&mint(&kid, &[]));
        assert_eq!(
            request
                .get("/api/cases")
                .add_header(rk.clone(), rv.clone())
                .await
                .status_code(),
            200,
            "read allowed for any authenticated caller"
        );
        assert_eq!(
            request
                .post("/api/cases")
                .add_header(rk, rv)
                .json(&housing_case())
                .await
                .status_code(),
            403,
            "write denied for a caller without access=write (policy 403)"
        );

        // Valid token, access=write ⇒ create allowed (reaches the handler
        // and does real DB work).
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        assert_eq!(
            request
                .post("/api/cases")
                .add_header(wk, wv)
                .json(&housing_case())
                .await
                .status_code(),
            200,
            "write allowed for access=write (reaches the handler)"
        );
    })
    .await;
}
