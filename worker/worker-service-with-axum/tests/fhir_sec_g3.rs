#![warn(clippy::pedantic)]

//! Record-level reads on worker's FHIR surface: `GET /fhir/Practitioner/{id}`
//! takes the same decision as `GET /api/workers/{id}`, and FHIR search
//! leaves out a worker the caller may not read, masks one under a `mask`
//! obligation, and cannot confirm a masked value.
//!
//! Its **own test binary**, like `tests/dedup_sec_g3.rs`: the flag, the key
//! set and the policy are process-wide `OnceLock`s read on first use, and
//! this test turns enforcement on. The handlers are called directly on one
//! [`AppState`], so the seeded workers and the search share one index.
//!
//! `#[ignore]`d — requires `PostgreSQL`. Run with
//! `cargo test --features paseto --test fhir_sec_g3 -- --ignored`.

#![cfg(feature = "paseto")]

mod common;

use axum::extract::Query;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use worker_service::api::fhir::handlers::{get_fhir_worker, search_fhir_workers};
use worker_service::api::rest::AppState;
use worker_service::api::rest::auth::{MaybeAuthUser, token_claims};
use worker_service::compliance::disclosure::AccessContext;
use worker_service::db::AuditContext;
use worker_service::models::{Gender, HumanName, Identifier, Worker};

/// Throwaway Ed25519 seed for test tokens; never used in production.
const SEED: [u8; 32] = [93; 32];
const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";

fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// A caller holding a signed `v4.public` token with no attributes.
async fn reader(kid: &str) -> MaybeAuthUser {
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": "55555555-5555-5555-5555-555555555555",
        "email": "reader@example.test", "name": "Reader",
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
    let token = builder.try_sign(&private).expect("sign");
    MaybeAuthUser(Some(token_claims(&token).await.expect("verify")))
}

/// Deny reading an inactive record; read a deceased one masked.
fn policy() -> Value {
    json!({
        "rules": [
            { "effect": "deny", "actions": ["read"],
              "when": { "resource.active": ["false"] } },
            { "effect": "allow", "actions": ["read"],
              "when": { "resource.deceased": ["true"] },
              "obligations": ["mask"] }
        ]
    })
}

fn worker(family: &str) -> Worker {
    Worker::new(
        HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec!["Avery".to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    )
}

/// Store `w` and index it.
async fn seed(state: &AppState, w: &Worker) -> Uuid {
    let created = state
        .worker_repository
        .create(w, &AuditContext::default())
        .await
        .expect("seed a worker");
    state
        .search_engine
        .index_worker(&created)
        .expect("index the worker");
    created.id
}

async fn body_of(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn search(state: &AppState, caller: MaybeAuthUser, name: &str) -> (StatusCode, Value) {
    body_of(
        search_fhir_workers(
            State(state.clone()),
            caller,
            AccessContext::internal(),
            Query(serde_json::from_value(json!({ "name": name })).expect("search params")),
        )
        .await,
    )
    .await
}

fn entry_ids(bundle: &Value) -> Vec<String> {
    bundle["entry"]
        .as_array()
        .expect("entry")
        .iter()
        .map(|e| e["resource"]["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test --features paseto --test fhir_sec_g3 -- --ignored`"]
async fn fhir_reads_take_the_record_level_decision() {
    let (keys, kid) = keys_and_kid();
    // Set before the state is built: read once into `OnceLock`s.
    // `set_var` is `unsafe` in edition 2024; single-threaded setup.
    unsafe {
        std::env::set_var("WORKER_REQUIRE_AUTH", "1");
        std::env::set_var("WORKER_PASETO_KEYS", keys.to_string());
        std::env::set_var("WORKER_ABAC_POLICY", policy().to_string());
    }
    let state = common::create_test_app_state().await;
    let caller = || reader(&kid);

    let family = format!("Fhir{}", &Uuid::new_v4().simple().to_string()[..12]);
    // A nine-digit social security number unique to this run, since
    // identifiers are unique table-wide.
    let digits: String = Uuid::new_v4()
        .as_u128()
        .to_string()
        .chars()
        .take(9)
        .collect();
    let readable = worker(&family);
    let mut denied = worker(&family);
    denied.active = false;
    let mut masked = worker(&family);
    masked.deceased = true;
    masked.identifiers.push(Identifier::ssn(digits.clone()));
    let readable_id = seed(&state, &readable).await.to_string();
    let denied_id = seed(&state, &denied).await;
    let masked_id = seed(&state, &masked).await;

    // Read: a denied worker is the REST read's 403, as an OperationOutcome.
    let (status, body) = body_of(
        get_fhir_worker(
            State(state.clone()),
            Path(denied_id),
            caller().await,
            AccessContext::internal(),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["resourceType"], "OperationOutcome", "{body}");
    assert_eq!(body["issue"][0]["code"], "forbidden", "{body}");

    // Read: a mask obligation returns the masked identifier.
    let (status, body) = body_of(
        get_fhir_worker(
            State(state.clone()),
            Path(masked_id),
            caller().await,
            AccessContext::internal(),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let shown = body.to_string();
    assert!(
        !shown.contains(&digits),
        "masked read hides the SSN: {body}"
    );

    // Search by name: the denied worker is left out and `total` counts what
    // is shown; the masked one is present.
    let (status, bundle) = search(&state, caller().await, &family).await;
    assert_eq!(status, StatusCode::OK, "{bundle}");
    let ids = entry_ids(&bundle);
    assert!(
        !ids.contains(&denied_id.to_string()),
        "denied worker left out: {bundle}"
    );
    assert!(ids.contains(&readable_id), "{bundle}");
    assert!(ids.contains(&masked_id.to_string()), "{bundle}");
    assert_eq!(bundle["total"].as_u64(), Some(ids.len() as u64), "{bundle}");
    assert!(
        !bundle.to_string().contains(&digits),
        "a search result never carries the masked SSN: {bundle}"
    );

    // Search by the masked SSN itself: the masked worker must not come back,
    // or the hit would confirm the hidden number.
    let (status, bundle) = search(&state, caller().await, &digits).await;
    assert_eq!(status, StatusCode::OK, "{bundle}");
    assert!(
        !entry_ids(&bundle).contains(&masked_id.to_string()),
        "a search on a masked value cannot confirm it: {bundle}"
    );
}
