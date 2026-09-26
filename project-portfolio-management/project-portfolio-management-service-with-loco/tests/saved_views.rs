//! Per-user saved views (T-28l), end to end over the real router with
//! real PASETO identities.
//!
//! Its **own test binary**, for the same reason `enforcement.rs` is:
//! `verifier` is a process-wide `OnceLock`, read once on first use —
//! sharing the `tests/requests/mod.rs` binary would race whichever
//! test happens to touch an authenticated route first.
//!
//! `#[ignore]`d — boots the app, so it needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`. Run with
//! `cargo test --test saved_views -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SEED: [u8; 32] = [31; 32];
const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";

/// The published Ed25519 key set plus its `kid`, derived the way the
/// verifier derives it (SHA-256 of the public key, base64url).
fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// Mint a `v4.public` token for `sub`.
fn mint(kid: &str, sub: &str) -> String {
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": sub,
        "email": format!("{sub}@example.com"), "name": sub,
        "iss": ISSUER, "aud": AUDIENCE,
        "exp": iat + 10_000_000_000_i64, "iat": iat,
        "sid": "test-sid", "attrs": {},
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

fn auth_header(token: &str) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    (
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    )
}

/// T-28l acceptance: two users on one route see their own views, and
/// a view is scoped to its route, never applied elsewhere. Plus: no
/// token is a `401` (a saved view with no owner makes no sense), and
/// one user cannot read or delete another's view (`404`, not `403` —
/// existence is not disclosed across users).
#[tokio::test]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test saved_views -- --ignored`"]
async fn saved_views_are_scoped_to_their_owner_and_their_route() {
    let (keys, kid) = keys_and_kid();
    // SAFETY: this binary is single-test, so no other test races the
    // process-wide verifier `OnceLock` it sets.
    unsafe {
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS", keys.to_string());
    }
    let (alice_key, alice_val) = auth_header(&mint(&kid, "alice"));
    let (bob_key, bob_val) = auth_header(&mint(&kid, "bob"));

    request::<App, _, _>(|request, _ctx| async move {
        // No token at all: a saved view with no owner makes no sense.
        assert_eq!(
            request
                .post("/api/saved-views")
                .json(&json!({ "route": "/plans", "name": "Mine" }))
                .await
                .status_code(),
            401
        );

        // Alice saves a view on /plans and a different one on /tasks.
        let alice_plans: Value = request
            .post("/api/saved-views")
            .add_header(alice_key.clone(), alice_val.clone())
            .json(&json!({
                "route": "/plans", "name": "My active plans",
                "filter": { "status": "active" }, "columns": ["name", "status"],
            }))
            .await
            .json();
        assert_eq!(alice_plans["route"], "/plans");
        let alice_plans_pid = alice_plans["pid"].as_str().expect("pid").to_string();

        request
            .post("/api/saved-views")
            .add_header(alice_key.clone(), alice_val.clone())
            .json(&json!({ "route": "/tasks", "name": "My open tasks" }))
            .await
            .assert_status_ok();

        // Bob saves his own view on the same /plans route.
        let bob_plans: Value = request
            .post("/api/saved-views")
            .add_header(bob_key.clone(), bob_val.clone())
            .json(&json!({ "route": "/plans", "name": "Bob's plans" }))
            .await
            .json();
        let bob_plans_pid = bob_plans["pid"].as_str().expect("pid").to_string();

        // Each user sees only their own views on /plans — never the
        // other's, even though both saved one on the same route.
        let alice_view: Value = request
            .get("/api/saved-views?route=/plans")
            .add_header(alice_key.clone(), alice_val.clone())
            .await
            .json();
        let alice_names: Vec<&str> = alice_view
            .as_array()
            .expect("array")
            .iter()
            .map(|v| v["name"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(alice_names, vec!["My active plans"]);

        let bob_view: Value = request
            .get("/api/saved-views?route=/plans")
            .add_header(bob_key.clone(), bob_val.clone())
            .await
            .json();
        let bob_names: Vec<&str> = bob_view
            .as_array()
            .expect("array")
            .iter()
            .map(|v| v["name"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(bob_names, vec!["Bob's plans"]);

        // A view is scoped to its route: Alice's /tasks view never
        // appears when she asks for /plans.
        assert!(
            alice_names.iter().all(|n| *n != "My open tasks"),
            "the /tasks view must not leak into the /plans listing"
        );

        // Bob cannot read or delete Alice's view: 404, not 403 — her
        // view's existence is not disclosed to him.
        assert_eq!(
            request
                .delete(&format!("/api/saved-views/{alice_plans_pid}"))
                .add_header(bob_key.clone(), bob_val.clone())
                .await
                .status_code(),
            404
        );
        // Alice's view is still there after Bob's failed delete.
        let alice_view_after: Value = request
            .get("/api/saved-views?route=/plans")
            .add_header(alice_key.clone(), alice_val.clone())
            .await
            .json();
        assert_eq!(alice_view_after.as_array().expect("array").len(), 1);

        // Alice can delete her own.
        request
            .delete(&format!("/api/saved-views/{alice_plans_pid}"))
            .add_header(alice_key.clone(), alice_val.clone())
            .await
            .assert_status_ok();
        let alice_view_gone: Value = request
            .get("/api/saved-views?route=/plans")
            .add_header(alice_key, alice_val)
            .await
            .json();
        assert!(alice_view_gone.as_array().expect("array").is_empty());

        // Bob's own view is untouched by any of the above.
        let bob_view_after: Value = request
            .get("/api/saved-views?route=/plans")
            .add_header(bob_key, bob_val)
            .await
            .json();
        assert_eq!(
            bob_view_after.as_array().expect("array")[0]["pid"],
            bob_plans_pid
        );
    })
    .await;
}
