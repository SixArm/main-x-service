//! The bulk links pull as a privileged read (SEC-G1), with enforcement
//! on. Its own binary: the enforcement flag and key set are read once.

#![cfg(feature = "paseto")]

mod common;

use axa::extract::Query;
use axum::extract::State;
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};
use worker_service::api::rest::auth::{MaybeAuthUser, token_claims};
use worker_service::api::rest::links::{BulkParams, bulk_links};

const SEED: [u8; 32] = [95; 32];

fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

async fn caller(kid: &str, attrs: Value) -> MaybeAuthUser {
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": "66666666-6666-6666-6666-666666666666",
        "email": "writer@example.test", "name": "Writer",
        "iss": "authentication-service", "aud": "main-x-service",
        "exp": iat + 10_000_000_000_i64, "iat": iat,
        "sid": "test-sid", "attrs": attrs,
    })
    .to_string();
    let key = Key::<64>::from(SigningKey::from_bytes(&SEED).to_keypair_bytes());
    let private = PasetoAsymmetricPrivateKey::<V4, Public>::from(&key);
    let mut builder = Paseto::<V4, Public>::builder();
    builder.set_payload(Payload::from(payload.as_str()));
    let footer = format!(r#"{{"kid":"{kid}"}}"#);
    builder.set_footer(Footer::from(footer.as_str()));
    let token = builder.try_sign(&private).expect("sign");
    MaybeAuthUser(Some(token_claims(&token).await.expect("verify")))
}

async fn status(state: &worker_service::api::rest::AppState, who: MaybeAuthUser) -> StatusCode {
    let params: BulkParams = serde_json::from_value(json!({})).unwrap();
    bulk_links(
        State(state.clone()),
        who,
        axa::client::ClientAddr(None),
        Query(params),
    )
    .await
    .status()
}

/// SEC-G1: the bulk pull surfaces every `same_identity` edge at once, so
/// it is a privileged read (`destructive`): the built-in default policy
/// refuses an ordinary caller and allows a machine peer or an admin. No
/// policy is configured here, so this pins the default.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test --test bulk_links_enforced -- --ignored`"]
async fn bulk_pull_is_a_privileged_read() {
    let (keys, kid) = keys_and_kid();
    // SAFETY: single-threaded setup, before the state reads them.
    unsafe {
        std::env::set_var("WORKER_REQUIRE_AUTH", "1");
        std::env::set_var("WORKER_PASETO_KEYS", keys.to_string());
        std::env::remove_var("WORKER_ABAC_POLICY");
    }
    let state = common::create_test_app_state().await;

    for (who, attrs) in [
        ("no attributes", json!({})),
        ("a writer", json!({ "access": ["write"] })),
    ] {
        assert_eq!(
            status(&state, caller(&kid, attrs).await).await,
            StatusCode::FORBIDDEN,
            "{who} may not dump every identity link"
        );
    }
    for (who, attrs) in [
        ("a machine peer", json!({ "svc": ["true"] })),
        ("an admin", json!({ "access": ["admin"] })),
    ] {
        assert_eq!(
            status(&state, caller(&kid, attrs).await).await,
            StatusCode::OK,
            "{who}"
        );
    }
    assert_eq!(
        status(&state, MaybeAuthUser(None)).await,
        StatusCode::UNAUTHORIZED,
        "no token"
    );
}
