//! End-to-end **blanket enforcement + ABAC** proof over the real router
//! with `EVENT_REQUIRE_AUTH` **on** — the "activation proof" (AU-1).
//!
//! The DB-free unit tests in `src/api/rest/auth.rs` pin the pure
//! `enforce` / policy-evaluation matrix. What they cannot show is that
//! the middleware is actually *wired* on the router the service serves,
//! and that a real request with (or without) a valid PASETO gets the
//! right status. Activation is the moment this service stops being open,
//! so it deserves a test that fails if the wiring is ever lost.
//!
//! Its **own test binary**, because `require_auth`, `policy` and
//! `verifier` are process-wide `OnceLock`s: the enforcement-*off*
//! integration suite in the same process would otherwise cache the flag
//! as off before this test sets it.
//!
//! `#[ignore]`d — this boots the app, so it needs PostgreSQL. Run with
//! `cargo test --test enforcement -- --ignored`.

#![cfg(feature = "paseto")]

use axa_test::prelude::*;
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use event_service::app::App;
use event_service::models::event::Event;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

/// Throwaway Ed25519 seed: mints test tokens and the matching published
/// key set in-process. Not a secret, never used in production.
const SEED: [u8; 32] = [23; 32];
const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";

/// The published key set (as the auth service would serve it) plus the
/// `kid` that selects the throwaway key.
fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// Mint a signed `v4.public` bearer carrying the given ABAC attributes.
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

/// A valid event payload with a unique name, so a create that reaches
/// the handler is not turned away as a duplicate.
///
/// Built by serializing `Thing::new`, not by hand. The wire type
/// requires every field the model declares without a serde default —
/// `id`, `active`, `created_at`, `updated_at`, `keywords`, … — all of
/// which the server owns and overwrites, so a hand-written body is a
/// game of "which field does it want next" and a `422` from the JSON
/// extractor when you lose. That contract is the same wart fixed in the
/// event service (`POST /api/events` demanded a `created_at` it then
/// discarded); it is recorded as QA-SERVER-FIELDS rather than fixed
/// here, because this change is about the auth guard and a payload
/// contract change belongs in its own three-part commit.
fn event_body() -> Value {
    let unique = uuid::Uuid::new_v4().simple().to_string();
    serde_json::to_value(Event::new(
        format!("{unique} Enforcement Event"),
        chrono::Utc::now(),
    ))
    .expect("serialize")
}

/// The `Authorization` value for a bearer token.
fn bearer(token: &str) -> axum::http::HeaderValue {
    format!("Bearer {token}").parse().unwrap()
}

/// `GET` the path with an optional bearer, returning the status.
async fn get(server: &TestServer, path: &str, token: Option<&str>) -> StatusCode {
    let mut request = server.get(path);
    if let Some(token) = token {
        request = request.add_header(axum::http::header::AUTHORIZATION, bearer(token));
    }
    request.await.status_code()
}

/// `POST` an event payload, returning the status **and** the body, so a
/// refusal reports the server's reason rather than only a status
/// mismatch.
async fn post_event_verbose(server: &TestServer, token: Option<&str>) -> (StatusCode, String) {
    let mut request = server.post("/api/events").json(&event_body());
    if let Some(token) = token {
        request = request.add_header(axum::http::header::AUTHORIZATION, bearer(token));
    }
    let response = request.await;
    (response.status_code(), response.text())
}

/// `POST` an event payload with an optional bearer, returning the status.
async fn post_event(server: &TestServer, token: Option<&str>) -> StatusCode {
    post_event_verbose(server, token).await.0
}

/// With enforcement on: public paths stay open, protected paths need a
/// token, and the default policy allows any authenticated caller to read
/// while requiring `access=write` to write.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_on_gates_the_real_router() {
    let (keys, kid) = keys_and_kid();
    let index =
        std::env::temp_dir().join(format!("event-enforcement-index-{}", std::process::id()));
    std::fs::create_dir_all(&index).expect("create search index dir");
    // Set BEFORE the router is built — the flag, the key set and the
    // policy are read into process-wide `OnceLock`s on first use.
    // `set_var` is `unsafe` in edition 2024; single-threaded setup.
    unsafe {
        std::env::set_var("EVENT_REQUIRE_AUTH", "1");
        std::env::set_var("SEARCH_INDEX_PATH", &index);
        std::env::set_var("EVENT_PASETO_KEYS", keys.to_string());
        // No EVENT_ABAC_POLICY ⇒ the built-in default policy: read
        // allow, mutation deny, `access=write` writes.
    }

    request::<App, _, _>(|server, _ctx| async move {
        assert_eq!(
            get(&server, "/api/health", None).await,
            StatusCode::OK,
            "a public path stays open with enforcement on"
        );
        assert_eq!(
            get(&server, "/metrics.prom", None).await,
            StatusCode::OK,
            "metrics stay scrapeable without a token"
        );

        assert_eq!(
            get(&server, "/api/events/search?q=anyone", None).await,
            StatusCode::UNAUTHORIZED,
            "a protected read without a token is 401"
        );
        assert_eq!(
            post_event(&server, None).await,
            StatusCode::UNAUTHORIZED,
            "a protected write without a token is 401"
        );

        // A garbage bearer is a 401, not a 500 — the guard must reject
        // malformed credentials rather than fall over on them.
        assert_eq!(
            get(&server, "/api/events/search?q=anyone", Some("not-a-token")).await,
            StatusCode::UNAUTHORIZED,
            "a malformed bearer is 401"
        );

        // Valid token, no attributes: the default policy allows the read and
        // denies the write — 403, distinct from the 401 above. The
        // 401/403 split is the contract in authorization-attributes.md §5.
        let reader = mint(&kid, &[]);
        assert_eq!(
            get(&server, "/api/events/search?q=anyone", Some(&reader)).await,
            StatusCode::OK,
            "any authenticated caller may read"
        );
        assert_eq!(
            post_event(&server, Some(&reader)).await,
            StatusCode::FORBIDDEN,
            "a caller without access=write is denied by policy, not by authentication"
        );

        // `access=write` reaches the handler and does real database work.
        let writer = mint(&kid, &[("access", &["write"])]);
        let (status, body) = post_event_verbose(&server, Some(&writer)).await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "access=write may create: {body}"
        );
    })
    .await;

    unsafe {
        std::env::remove_var("EVENT_REQUIRE_AUTH");
        std::env::remove_var("EVENT_PASETO_KEYS");
    }
}
