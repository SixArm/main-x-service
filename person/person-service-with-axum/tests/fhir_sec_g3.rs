#![warn(clippy::pedantic)]

//! Record-level reads on the FHIR surface: `GET /fhir/Patient/{id}` (and
//! the `Person` alias) take the same decision REST `GET /api/persons/{id}`
//! takes, and FHIR search applies the per-record read visibility REST
//! search does. A denied record is a `403` `OperationOutcome` on read and
//! absent from a search `Bundle`, whose `total` counts only what is shown;
//! a `mask`-obligated record is rendered masked, and a search on a value
//! masking hides does not return it.
//!
//! Its **own test binary**: the flag, the key set and the policy are
//! process-wide `OnceLock`s read on first use, and this test turns
//! enforcement on.
//!
//! `#[ignore]`d — requires `PostgreSQL`. Run with
//! `cargo test --test fhir_sec_g3 -- --ignored`.

#![cfg(feature = "paseto")]

mod common;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::NaiveDate;
use ed25519_dalek::SigningKey;
use person_service::api::fhir::handlers::{
    FhirSearchParams, get_fhir_patient, get_fhir_person_alias, search_fhir_patients,
};
use person_service::api::rest::AppState;
use person_service::api::rest::auth::{MaybeAuthUser, claims_from_token};
use person_service::compliance::disclosure::AccessContext;
use person_service::db::AuditContext;
use person_service::models::{Gender, HumanName, Identifier, Person};
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Throwaway Ed25519 seed for test tokens; never used in production.
const SEED: [u8; 32] = [91; 32];
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
        "sub": "44444444-4444-4444-4444-444444444444",
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
    MaybeAuthUser(Some(claims_from_token(&token).await.expect("verify")))
}

/// Deny reading an inactive record; read a deceased one masked. (The
/// duplicate path loads candidates by id, so an inactive record still
/// reaches it; `managing_org` would need a stored organization.)
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

fn search(name: Option<String>, identifier: Option<String>) -> FhirSearchParams {
    FhirSearchParams {
        name,
        family: None,
        given: None,
        identifier,
        birth_date: None,
        gender: None,
        count: Some(50),
    }
}

/// The ids in a searchset `Bundle`, after checking `total` counts them.
fn bundle_ids(bundle: &Value) -> Vec<String> {
    let entries = bundle["entry"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        bundle["total"].as_u64(),
        Some(entries.len() as u64),
        "total counts only what is shown: {bundle}"
    );
    entries
        .iter()
        .filter_map(|e| e["resource"]["id"].as_str().map(str::to_owned))
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test --test fhir_sec_g3 -- --ignored`"]
async fn fhir_read_and_search_take_the_record_level_decision() {
    let (keys, kid) = keys_and_kid();
    // Set before the state is built: read once into `OnceLock`s.
    // `set_var` is `unsafe` in edition 2024; single-threaded setup.
    unsafe {
        std::env::set_var("PERSON_REQUIRE_AUTH", "1");
        std::env::set_var("PERSON_PASETO_KEYS", keys.to_string());
        std::env::set_var("PERSON_ABAC_POLICY", policy().to_string());
    }
    let state = common::create_test_app_state().await;
    let caller = || reader(&kid);

    let family = format!("Fhirg{}", &Uuid::new_v4().simple().to_string()[..12]);
    // A nine-digit SSN unique to the run, one index token.
    let ssn = format!("{:09}", Uuid::new_v4().as_u128() % 1_000_000_000);
    let full = person(&family);
    let mut denied = person(&family);
    denied.active = false;
    let mut masked = person(&family);
    masked.deceased = true;
    masked.identifiers.push(Identifier::ssn(ssn.clone()));
    let full_id = seed(&state, &full).await.to_string();
    let denied_id = seed(&state, &denied).await.to_string();
    let masked_id = seed(&state, &masked).await.to_string();

    // Read: a denied record answers REST's 403, as an OperationOutcome,
    // on both the Patient and the Person alias.
    let (status, body) = body_of(
        get_fhir_patient(
            State(state.clone()),
            Path(denied_id.clone()),
            caller().await,
            AccessContext::from_parts(None, None, None),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    assert_eq!(body["resourceType"], "OperationOutcome", "{body}");
    assert_eq!(body["issue"][0]["code"], "forbidden", "{body}");
    let (status, _) = body_of(
        get_fhir_person_alias(
            State(state.clone()),
            Path(denied_id.clone()),
            caller().await,
            AccessContext::from_parts(None, None, None),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Read: a mask obligation renders the masked person.
    let (status, body) = body_of(
        get_fhir_patient(
            State(state.clone()),
            Path(masked_id.clone()),
            caller().await,
            AccessContext::from_parts(None, None, None),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        !body.to_string().contains(&ssn),
        "the masked read hides the SSN: {body}"
    );

    // Search by name: the denied record is absent, the other two shown.
    let (status, bundle) = body_of(
        search_fhir_patients(
            State(state.clone()),
            Query(search(Some(family.clone()), None)),
            caller().await,
            AccessContext::from_parts(None, None, None),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{bundle}");
    let ids = bundle_ids(&bundle);
    assert!(
        !ids.contains(&denied_id),
        "denied record concealed: {bundle}"
    );
    assert!(ids.contains(&full_id), "readable record shown: {bundle}");
    assert!(ids.contains(&masked_id), "masked record shown: {bundle}");
    assert!(
        !bundle.to_string().contains(&ssn),
        "masked in search: {bundle}"
    );

    // Search by the hidden SSN: the masked record must not come back, or
    // the hit would confirm the value masking hides.
    let (status, bundle) = body_of(
        search_fhir_patients(
            State(state.clone()),
            Query(search(None, Some(ssn.clone()))),
            caller().await,
            AccessContext::from_parts(None, None, None),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{bundle}");
    assert!(
        !bundle_ids(&bundle).contains(&masked_id),
        "a search on a masked value does not confirm it: {bundle}"
    );
}
