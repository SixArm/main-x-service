//! FHIR writes take the same record-level decision as REST writes
//! (security invariant 5, `agents/share/fhir.md` §8).
//!
//! `PUT` and `DELETE /api/workers/{id}` load the stored worker and run
//! `authorize_record` on it before writing; `PUT` and `DELETE
//! /fhir/Practitioner/{id}` went straight to the repository, so a
//! `resource.*` policy refusing writes to, say, an inactive worker was
//! enforced on one surface and not the other, and an unknown id was a
//! `500` rather than `404`. Here a policy refuses writes to an inactive
//! worker and allows them otherwise. DB-gated; its own binary, since the
//! policy is read once per process.

#![cfg(feature = "paseto")]

mod common;

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
use worker_service::api::fhir::handlers::{delete_fhir_worker, update_fhir_worker};
use worker_service::api::rest::AppState;
use worker_service::api::rest::auth::{MaybeAuthUser, token_claims};
use worker_service::db::AuditContext;
use worker_service::models::{Gender, HumanName, Worker};

const SEED: [u8; 32] = [94; 32];

fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

async fn writer(kid: &str) -> MaybeAuthUser {
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": "66666666-6666-6666-6666-666666666666",
        "email": "writer@example.test", "name": "Writer",
        "iss": "authentication-service", "aud": "main-x-service",
        "exp": iat + 10_000_000_000_i64, "iat": iat,
        "sid": "test-sid", "attrs": { "access": ["write"] },
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

/// Refuse writes and deletes to an inactive worker; allow them otherwise.
fn policy() -> Value {
    json!({
        "rules": [
            { "effect": "deny", "actions": ["write", "delete"],
              "when": { "resource.active": ["false"] } },
            { "effect": "allow", "actions": ["write", "delete"], "when": {} }
        ]
    })
}

fn worker(family: &str, active: bool) -> Worker {
    let mut w = Worker::new(
        HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec!["Rhian".to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    );
    w.active = active;
    w
}

async fn seed(state: &AppState, w: &Worker) -> Uuid {
    state
        .worker_repository
        .create(w, &AuditContext::default())
        .await
        .expect("seed a worker")
        .id
}

fn practitioner(family: &str) -> axum::body::Bytes {
    json!({
        "resourceType": "Practitioner",
        "name": [{ "use": "official", "family": family, "given": ["Changed"] }],
        "gender": "female"
    })
    .to_string()
    .into()
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

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test --features paseto --test fhir_write_abac -- --ignored`"]
async fn fhir_writes_take_the_record_level_decision() {
    let (keys, kid) = keys_and_kid();
    // SAFETY: single-threaded setup, before the state reads them.
    unsafe {
        std::env::set_var("WORKER_REQUIRE_AUTH", "1");
        std::env::set_var("WORKER_PASETO_KEYS", keys.to_string());
        std::env::set_var("WORKER_ABAC_POLICY", policy().to_string());
    }
    let state = common::create_test_app_state().await;
    let family = format!(
        "Fhirwrite{}",
        Uuid::new_v4()
            .simple()
            .to_string()
            .replace(char::is_numeric, "q")
    );
    let inactive = seed(&state, &worker(&family, false)).await;
    let active = seed(&state, &worker(&family, true)).await;
    let client = || axa::client::ClientAddr(None);

    // Refused on the stored record, and the record is left as it was.
    let (status, body) = body_of(
        update_fhir_worker(
            State(state.clone()),
            Path(inactive),
            writer(&kid).await,
            client(),
            practitioner(&family),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["resourceType"], "OperationOutcome", "{body}");
    let unchanged = state
        .worker_repository
        .get_by_id(&inactive)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.name.given, vec!["Rhian".to_string()]);

    let (status, body) = body_of(
        delete_fhir_worker(
            State(state.clone()),
            Path(inactive),
            writer(&kid).await,
            client(),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert!(
        state
            .worker_repository
            .get_by_id(&inactive)
            .await
            .unwrap()
            .is_some(),
        "not deleted"
    );

    // Allowed where the policy allows.
    let (status, body) = body_of(
        update_fhir_worker(
            State(state.clone()),
            Path(active),
            writer(&kid).await,
            client(),
            practitioner(&family),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _) = body_of(
        delete_fhir_worker(
            State(state.clone()),
            Path(active),
            writer(&kid).await,
            client(),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // An unknown id is `404 not-found`, not a database error.
    let missing = Uuid::new_v4();
    let (status, body) = body_of(
        update_fhir_worker(
            State(state.clone()),
            Path(missing),
            writer(&kid).await,
            client(),
            practitioner(&family),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["issue"][0]["code"], "not-found", "{body}");
    let (status, _) = body_of(
        delete_fhir_worker(
            State(state.clone()),
            Path(missing),
            writer(&kid).await,
            client(),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let conn = common::db().await;
    common::purge_record(&conn, inactive).await;
    common::purge_record(&conn, active).await;
}
