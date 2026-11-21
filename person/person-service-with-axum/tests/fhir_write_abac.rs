//! SEC-G10: FHIR writes take the record-level decision and the native
//! validation, as `PUT` / `DELETE /api/persons/{id}` and `POST` do. Its
//! own binary: the policy is read once per process.

#![cfg(feature = "paseto")]

mod common;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::NaiveDate;
use ed25519_dalek::SigningKey;
use person_service::api::fhir::handlers::{
    create_fhir_patient, delete_fhir_patient, update_fhir_patient,
};
use person_service::api::rest::AppState;
use person_service::api::rest::auth::{MaybeAuthUser, claims_from_token};
use person_service::db::AuditContext;
use person_service::models::{Gender, HumanName, Person};
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Throwaway Ed25519 seed for test tokens; never used in production.
const SEED: [u8; 32] = [92; 32];
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

/// A caller holding a signed `v4.public` token with `access=write`.
async fn writer(kid: &str) -> MaybeAuthUser {
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": "44444444-4444-4444-4444-444444444444",
        "email": "reader@example.test", "name": "Reader",
        "iss": ISSUER, "aud": AUDIENCE,
        "exp": iat + 10_000_000_000_i64, "iat": iat,
        "sid": "test-sid", "attrs": { "access": ["write"] },
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
    MaybeAuthUser(Some(claims_from_token(&token).await.expect("verify")))
}

/// Refuse writes and deletes to an inactive person; allow them otherwise.
fn policy() -> Value {
    json!({
        "rules": [
            { "effect": "deny", "actions": ["write", "delete"],
              "when": { "resource.active": ["false"] } },
            { "effect": "allow", "actions": ["write", "delete"], "when": {} }
        ]
    })
}

fn person(family: &str) -> Person {
    let mut p = Person::new(
        HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec!["Avery".to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Female,
    );
    p.birth_date = NaiveDate::from_ymd_opt(1981, 6, 14);
    p
}

/// Store `p` and index it, past the create-time duplicate check. Family
/// names stay short: the index drops a token longer than 40 characters.
async fn seed(state: &AppState, p: &Person) -> Uuid {
    let created = state
        .person_repository
        .create(p, &AuditContext::default())
        .await
        .expect("seed a person");
    state
        .search_engine
        .index_person(&created)
        .expect("index the person");
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

fn patient(family: &str) -> axum::body::Bytes {
    json!({
        "resourceType": "Patient",
        "name": [{ "use": "official", "family": family, "given": ["Changed"] }],
        "gender": "female",
        "birthDate": "1981-06-14"
    })
    .to_string()
    .into()
}

async fn put(
    state: &AppState,
    kid: &str,
    id: Uuid,
    body: axum::body::Bytes,
) -> (StatusCode, Value) {
    body_of(
        update_fhir_patient(
            State(state.clone()),
            Path(id.to_string()),
            writer(kid).await,
            axa::client::ClientAddr(None),
            body,
        )
        .await,
    )
    .await
}

async fn del(state: &AppState, kid: &str, id: Uuid) -> (StatusCode, Value) {
    body_of(
        delete_fhir_patient(
            State(state.clone()),
            Path(id.to_string()),
            writer(kid).await,
            axa::client::ClientAddr(None),
        )
        .await,
    )
    .await
}

/// SEC-G10: FHIR writes take the record-level decision and the native
/// validation, as the REST writes do. Without the fix a `PUT` on an
/// inactive person returned `200` and changed it.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test --test fhir_write_abac -- --ignored`"]
async fn fhir_writes_take_the_record_level_decision_and_validation() {
    let (keys, kid) = keys_and_kid();
    // SAFETY: single-threaded setup, before the state reads them.
    unsafe {
        std::env::set_var("PERSON_REQUIRE_AUTH", "1");
        std::env::set_var("PERSON_PASETO_KEYS", keys.to_string());
        std::env::set_var("PERSON_ABAC_POLICY", policy().to_string());
    }
    let state = common::create_test_app_state().await;
    let family = format!(
        "Fhirw{}",
        Uuid::new_v4()
            .simple()
            .to_string()
            .replace(char::is_numeric, "q")
    );
    let mut inactive_person = person(&family);
    inactive_person.active = false;
    let inactive = seed(&state, &inactive_person).await;
    let active = seed(&state, &person(&family)).await;
    let (status, body) = put(&state, &kid, inactive, patient(&family)).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["resourceType"], "OperationOutcome", "{body}");
    let unchanged = state
        .person_repository
        .get_by_id(&inactive)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(unchanged.name.given, vec!["Avery".to_string()]);
    let (status, body) = del(&state, &kid, inactive).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");

    let (status, body) = put(&state, &kid, active, patient(&family)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Validation: an over-long family name (SEC-M1) is a 422, on update and
    // create alike.
    let long = "x".repeat(5000);
    let (status, body) = put(&state, &kid, active, patient(&long)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["issue"][0]["code"], "processing", "{body}");
    let (status, body) = body_of(
        create_fhir_patient(
            State(state.clone()),
            writer(&kid).await,
            axa::client::ClientAddr(None),
            patient(&long),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");

    let (status, _) = del(&state, &kid, active).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let missing = Uuid::new_v4();
    let (status, body) = put(&state, &kid, missing, patient(&family)).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, _) = del(&state, &kid, missing).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let conn = common::db().await;
    common::purge_record(&conn, inactive).await;
    common::purge_record(&conn, active).await;
}
