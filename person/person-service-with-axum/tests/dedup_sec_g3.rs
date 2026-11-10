#![warn(clippy::pedantic)]

//! SEC-G3 on the duplicate paths: `POST /api/persons/check-duplicates`,
//! `POST /api/persons/match` and the create-time `409` apply the same per-record read visibility
//! search does. A candidate the caller may not read is absent, and does
//! not make a create a `409`; a `mask`-obligated candidate comes back
//! masked, with only the evidence aggregate.
//!
//! Its **own test binary**, like `tests/grpc_sec_g3.rs`: the flag, the
//! key set and the policy are process-wide `OnceLock`s read on first use,
//! and this test turns enforcement on.
//!
//! The handlers are called directly on one [`AppState`], so the seeded
//! candidates and the duplicate check share one search index.
//!
//! `#[ignore]`d — requires `PostgreSQL`. Run with
//! `cargo test --test dedup_sec_g3 -- --ignored`.

#![cfg(feature = "paseto")]

mod common;

use axa::client::ClientAddr;
use axa::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::NaiveDate;
use ed25519_dalek::SigningKey;
use person_service::api::rest::AppState;
use person_service::api::rest::auth::{MaybeAuthUser, claims_from_token};
use person_service::api::rest::handlers::{
    MatchRequest, check_duplicates, create_person, match_person,
};
use person_service::db::AuditContext;
use person_service::models::{Gender, HumanName, Person};
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

fn candidate_ids(matches: &Value) -> Vec<String> {
    matches
        .as_array()
        .expect("potential_matches")
        .iter()
        .map(|m| m["person"]["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DB_URL); run with `cargo test --test dedup_sec_g3 -- --ignored`"]
async fn duplicate_paths_conceal_denied_and_mask_obligated_candidates() {
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

    // Two candidates for one probe: one the caller may not read, one it
    // may read only masked.
    let family = format!("Secg{}", &Uuid::new_v4().simple().to_string()[..12]);
    let mut denied = person(&family);
    denied.active = false;
    let mut masked = person(&family);
    masked.deceased = true;
    masked.tax_id = Some("987654321".to_string());
    let denied_id = seed(&state, &denied).await.to_string();
    let masked_id = seed(&state, &masked).await.to_string();

    // check-duplicates: the denied candidate is absent, the masked one is
    // masked and carries only the evidence aggregate.
    let (status, body) = body_of(
        check_duplicates(State(state.clone()), caller().await, Json(person(&family)))
            .await
            .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let ids = candidate_ids(&body["potential_matches"]);
    assert!(
        !ids.contains(&denied_id),
        "denied candidate concealed: {body}"
    );
    assert!(ids.contains(&masked_id), "masked candidate present: {body}");
    let shown = body["potential_matches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["person"]["id"] == masked_id.as_str())
        .unwrap();
    assert_eq!(shown["person"]["tax_id"], "*****4321", "{shown}");
    assert!(
        shown["evidence"]["components"]
            .as_array()
            .is_none_or(Vec::is_empty),
        "masked candidate keeps only the evidence aggregate: {shown}"
    );

    // match: the same disposition.
    let (status, body) = body_of(
        match_person(
            State(state.clone()),
            caller().await,
            Json(MatchRequest {
                person: person(&family),
                threshold: None,
                limit: 10,
            }),
        )
        .await
        .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let ids = candidate_ids(&body["matches"]);
    assert!(
        !ids.contains(&denied_id),
        "denied candidate concealed: {body}"
    );
    assert!(ids.contains(&masked_id), "{body}");
    assert_eq!(body["total"].as_u64(), Some(ids.len() as u64), "{body}");

    // The 409 on create shows the same candidates.
    let (status, body) = body_of(
        create_person(
            State(state.clone()),
            caller().await,
            ClientAddr(None),
            Json(person(&family)),
        )
        .await
        .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let ids = candidate_ids(&body["errors"]["potential_matches"]);
    assert!(
        !ids.contains(&denied_id),
        "denied candidate concealed: {body}"
    );
    assert!(ids.contains(&masked_id), "{body}");

    // A probe whose only candidate is concealed is not a 409: the caller
    // must not learn the record exists.
    let lone = format!("Secglone{}", &Uuid::new_v4().simple().to_string()[..12]);
    let mut hidden = person(&lone);
    hidden.active = false;
    seed(&state, &hidden).await;
    let (status, body) = body_of(
        create_person(
            State(state.clone()),
            caller().await,
            ClientAddr(None),
            Json(person(&lone)),
        )
        .await
        .into_response(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
}
