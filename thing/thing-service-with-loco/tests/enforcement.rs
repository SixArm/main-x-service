//! End-to-end **blanket enforcement + ABAC** proof over the real router
//! with `THING_REQUIRE_AUTH` **on** — the "activation proof" (AU-1).
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
//! `#[ignore]`d — the router opens a database connection. Run with
//! `cargo test --test enforcement -- --ignored`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};
use thing_service::api::rest::{AppState, create_router};
use thing_service::config::Config;
use thing_service::db::create_connection;
use thing_service::matching::ThingMatcher;
use thing_service::models::thing::Thing;
use thing_service::search::SearchEngine;
use tower::ServiceExt;

/// Throwaway Ed25519 seed: mints test tokens and the matching published
/// key set in-process. Not a secret, never used in production.
const SEED: [u8; 32] = [19; 32];
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

/// Build the real router against the environment-configured database and
/// a temp search index.
///
/// This crate has no shared HTTP test harness — its `tests/` are library
/// tests over pure functions — so the activation proof brings its own.
/// It is deliberately the **production** `create_router`, since what is
/// being proved is that the middleware is wired onto the router the
/// service actually serves.
async fn test_router() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    let search_engine = SearchEngine::new(&config.search.index_path).expect("search engine");
    let matcher = ThingMatcher::new(&config.matching);
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection — set DATABASE_URL to a running, migrated database");
    create_router(AppState::new(db, search_engine, matcher, config))
}

/// A valid thing payload with a unique name, so a create that reaches
/// the handler is not turned away as a duplicate.
///
/// Built by serializing `Thing::new`, not by hand — not because a
/// hand-written body would fail (QA-SERVER-FIELDS made every
/// server-owned field — `id`, `is_deleted`, `created_at`, `updated_at`,
/// `alternate_names`, … — optional on the wire; see
/// `tests/api_integration_test.rs` for the hand-written-body coverage),
/// but because this helper only needs a valid, unique record and
/// `Thing::new` is the least ceremony way to get one.
fn thing_body() -> Value {
    let unique = uuid::Uuid::new_v4().simple().to_string();
    serde_json::to_value(Thing::new(&format!("{unique} Enforcement Thing"))).expect("serialize")
}

/// `GET` the path with an optional bearer, returning the status.
async fn get(app: &axum::Router, path: &str, token: Option<&str>) -> StatusCode {
    let mut builder = Request::builder().method("GET").uri(path);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

/// `POST` a thing payload, returning the status **and** the body, so a
/// refusal reports the server's reason rather than only a status
/// mismatch.
async fn post_thing_verbose(app: &axum::Router, token: Option<&str>) -> (StatusCode, String) {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/things")
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(
            builder
                .body(Body::from(serde_json::to_vec(&thing_body()).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// `POST` a thing payload with an optional bearer, returning the status.
async fn post_thing(app: &axum::Router, token: Option<&str>) -> StatusCode {
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/things")
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(
            builder
                .body(Body::from(serde_json::to_vec(&thing_body()).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

/// With enforcement on: public paths stay open, protected paths need a
/// token, and the default policy allows any authenticated caller to read
/// while requiring `access=write` to write.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_on_gates_the_real_router() {
    let (keys, kid) = keys_and_kid();
    // Set BEFORE the router is built — the flag, the key set and the
    // policy are read into process-wide `OnceLock`s on first use.
    // `set_var` is `unsafe` in edition 2024; single-threaded setup.
    unsafe {
        std::env::set_var("THING_REQUIRE_AUTH", "1");
        std::env::set_var("THING_PASETO_KEYS", keys.to_string());
        // No THING_ABAC_POLICY ⇒ the built-in default policy: read
        // allow, mutation deny, `access=write` writes.
    }

    let app = test_router().await;

    assert_eq!(
        get(&app, "/api/health", None).await,
        StatusCode::OK,
        "a public path stays open with enforcement on"
    );
    assert_eq!(
        get(&app, "/metrics.prom", None).await,
        StatusCode::OK,
        "metrics stay scrapeable without a token"
    );

    assert_eq!(
        get(&app, "/api/things/search?q=anyone", None).await,
        StatusCode::UNAUTHORIZED,
        "a protected read without a token is 401"
    );
    assert_eq!(
        post_thing(&app, None).await,
        StatusCode::UNAUTHORIZED,
        "a protected write without a token is 401"
    );

    // A garbage bearer is a 401, not a 500 — the guard must reject
    // malformed credentials rather than fall over on them.
    assert_eq!(
        get(&app, "/api/things/search?q=anyone", Some("not-a-token")).await,
        StatusCode::UNAUTHORIZED,
        "a malformed bearer is 401"
    );

    // Valid token, no attributes: the default policy allows the read and
    // denies the write — 403, distinct from the 401 above. The
    // 401/403 split is the contract in authorization-attributes.md §5.
    let reader = mint(&kid, &[]);
    assert_eq!(
        get(&app, "/api/things/search?q=anyone", Some(&reader)).await,
        StatusCode::OK,
        "any authenticated caller may read"
    );
    assert_eq!(
        post_thing(&app, Some(&reader)).await,
        StatusCode::FORBIDDEN,
        "a caller without access=write is denied by policy, not by authentication"
    );

    // `access=write` reaches the handler and does real database work.
    let writer = mint(&kid, &[("access", &["write"])]);
    let (status, body) = post_thing_verbose(&app, Some(&writer)).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "access=write may create: {body}"
    );

    unsafe {
        std::env::remove_var("THING_REQUIRE_AUTH");
        std::env::remove_var("THING_PASETO_KEYS");
    }
}
