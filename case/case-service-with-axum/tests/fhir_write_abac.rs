//! SEC-G10: FHIR writes take the record-level decision and the native
//! validation, as `PUT` / `DELETE /api/cases/{pid}` and `POST /api/cases`
//! do. Before the fix, `PUT` and `DELETE /fhir/Task/{id}` skipped
//! `authorize_record` on the stored case, and FHIR create and update skipped
//! validation (including the SEC-M1 input-size caps). A policy here refuses
//! writes to a closed case; its own binary, since the policy is read once.

#![cfg(feature = "paseto")]

use axa_test::prelude::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use case_service::app::App;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

const SEED: [u8; 32] = [9; 32];
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

fn auth_header(token: &str) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    (
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    )
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with `cargo test --test fhir_write_abac -- --ignored`"]
async fn fhir_writes_take_the_record_level_decision_and_validation() {
    let (keys, kid) = keys_and_kid();
    let policy = json!({
        "rules": [
            { "effect": "deny", "actions": ["write", "delete"],
              "when": { "resource.status": ["closed"] } },
            { "effect": "allow", "actions": ["write", "delete"], "when": { "access": ["write"] } }
        ]
    });
    // SAFETY: single-threaded, before the app boots.
    unsafe {
        std::env::set_var(
            "CASE_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!("case-fhir-write-index-{}", std::process::id())),
        );
        std::env::set_var("CASE_SEARCH_BOOT_REINDEX", "0");
        std::env::set_var("CASE_REQUIRE_AUTH", "1");
        std::env::set_var("CASE_PASETO_KEYS", keys.to_string());
        std::env::set_var("CASE_ABAC_POLICY", policy.to_string());
    }

    request::<App, _, _>(|request, _ctx| async move {
        let writer = || auth_header(&mint(&kid, &[("access", &["write"])]));
        let create = |title: &str, status: &str| {
            json!({ "title": title, "agency_id": "dwp", "status": status })
        };
        let pid_of = |text: String| -> String {
            serde_json::from_str::<Value>(&text).unwrap()["pid"].as_str().unwrap().to_string()
        };
        let (k, v) = writer();
        let closed = pid_of(
            request.post("/api/cases").add_header(k, v).json(&create("Closed case", "Closed")).await.text(),
        );
        let (k, v) = writer();
        let open = pid_of(
            request.post("/api/cases").add_header(k, v).json(&create("Open case", "Open")).await.text(),
        );
        let task = |description: &str| {
            json!({ "resourceType": "Task", "status": "in-progress", "intent": "order",
                    "description": description })
        };

        // A closed case is refused on FHIR as on REST, and left as it was.
        let (k, v) = writer();
        let r = request.put(&format!("/fhir/Task/{closed}")).add_header(k, v).json(&task("Rewritten")).await;
        assert_eq!(r.status_code(), 403, "{}", r.text());
        assert_eq!(serde_json::from_str::<Value>(&r.text()).unwrap()["resourceType"], "OperationOutcome");
        let (k, v) = writer();
        let r = request.delete(&format!("/fhir/Task/{closed}")).add_header(k, v).await;
        assert_eq!(r.status_code(), 403, "{}", r.text());
        let (k, v) = writer();
        let still = request.get(&format!("/api/cases/{closed}")).add_header(k, v).await;
        assert_eq!(still.status_code(), 200);
        assert!(still.text().contains("Closed case"), "unchanged: {}", still.text());

        // An open case may be updated and deleted.
        let (k, v) = writer();
        let r = request.put(&format!("/fhir/Task/{open}")).add_header(k, v).json(&task("Updated")).await;
        assert_eq!(r.status_code(), 200, "{}", r.text());

        // Validation applies to FHIR writes: an over-long title (SEC-M1) is a
        // 422 with an OperationOutcome, on create and update alike.
        let long = "x".repeat(5000);
        let (k, v) = writer();
        let r = request.post("/fhir/Task").add_header(k, v).json(&task(&long)).await;
        assert_eq!(r.status_code(), 422, "{}", r.text());
        assert_eq!(serde_json::from_str::<Value>(&r.text()).unwrap()["issue"][0]["code"], "processing");
        let (k, v) = writer();
        let r = request.put(&format!("/fhir/Task/{open}")).add_header(k, v).json(&task(&long)).await;
        assert_eq!(r.status_code(), 422, "{}", r.text());

        let (k, v) = writer();
        let r = request.delete(&format!("/fhir/Task/{open}")).add_header(k, v).await;
        assert_eq!(r.status_code(), 204, "{}", r.text());
    })
    .await;
}
