//! A caller who may read a case only masked must not learn a value the
//! masked view hides from whether a search returns the case (CS-FH1).
//! Exact REST search reads identifiers and subjects, and FHIR search
//! matched `identifier` against the real record; both now answer a masked
//! caller from the fields the masked view keeps, while an unmasked caller
//! still finds the case by its identifier.
//!
//! Its own test binary, because the auth flag and policy are read once per
//! process. `#[ignore]`d: boots the app against PostgreSQL. Run with
//! `cargo test --features paseto --test masked_search -- --ignored`.
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

/// The pids a search response lists.
fn pids(body: &Value) -> Vec<String> {
    body.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| r["pid"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The ids in a FHIR searchset `Bundle`.
fn bundle_ids(body: &Value) -> Vec<String> {
    body["entry"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|e| e["resource"]["id"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with `cargo test --features paseto --test masked_search -- --ignored`"]
async fn a_masked_caller_cannot_find_a_case_by_a_hidden_value() {
    let (keys, kid) = keys_and_kid();
    // `dept=partner` reads every case masked; everyone else reads in full.
    let policy = json!({
        "rules": [
            { "effect": "allow", "actions": ["read"],
              "when": { "dept": ["partner"] }, "obligations": ["mask"] },
            { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
        ]
    });
    unsafe {
        std::env::set_var(
            "CASE_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!("case-masked-search-{}", std::process::id())),
        );
        std::env::set_var("CASE_SEARCH_BOOT_REINDEX", "0");
        std::env::set_var("CASE_REQUIRE_AUTH", "1");
        std::env::set_var("CASE_PASETO_KEYS", keys.to_string());
        std::env::set_var("CASE_ABAC_POLICY", policy.to_string());
    }

    request::<App, _, _>(|request, _ctx| async move {
        let tag = uuid::Uuid::new_v4().simple().to_string();
        let docket = format!("DK{tag}");
        let title_word = format!("Msk{}", &tag[..12]);
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let created = request
            .post("/api/cases")
            .add_header(wk, wv)
            .json(&json!({
                "title": format!("{title_word} appeal"),
                "agency_id": "dwp",
                "identifiers": [{ "scheme": "Docket", "value": docket }],
            }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());
        let pid = serde_json::from_str::<Value>(&created.text()).unwrap()["pid"]
            .as_str()
            .expect("pid")
            .to_string();

        let full = mint(&kid, &[]);
        let partner = mint(&kid, &[("dept", &["partner"])]);
        let get = |token: String, uri: String| {
            let request = &request;
            async move {
                let (k, v) = auth_header(&token);
                let response = request.get(&uri).add_header(k, v).await;
                assert_eq!(response.status_code(), 200, "{uri}: {}", response.text());
                serde_json::from_str::<Value>(&response.text()).unwrap()
            }
        };

        // REST exact search on the hidden identifier.
        let by_docket = format!("/api/cases/search?q={docket}");
        assert!(
            pids(&get(full.clone(), by_docket.clone()).await).contains(&pid),
            "a full reader finds the case by its docket"
        );
        assert!(
            !pids(&get(partner.clone(), by_docket).await).contains(&pid),
            "a masked reader must not learn the docket from a hit"
        );
        // The kept fields still answer a masked reader.
        let by_title = format!("/api/cases/search?q={title_word}");
        assert!(
            pids(&get(partner.clone(), by_title).await).contains(&pid),
            "a masked reader still finds the case by its title"
        );

        // FHIR search on the hidden identifier.
        let fhir = format!("/fhir/Task?identifier={docket}");
        assert!(
            bundle_ids(&get(full, fhir.clone()).await).contains(&pid),
            "a full reader's FHIR search finds the case by its docket"
        );
        assert!(
            !bundle_ids(&get(partner, fhir).await).contains(&pid),
            "a masked reader's FHIR search must not confirm the docket"
        );
    })
    .await;
}
