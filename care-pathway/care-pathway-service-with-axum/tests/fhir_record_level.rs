//! CP-FH1: the FHIR surface takes the record-level read decision the REST
//! read takes (`agents/share/fhir.md` §8). With `CARE_PATHWAY_REQUIRE_AUTH`
//! on, `GET /fhir/PlanDefinition/{id}` answers a denied pathway with the
//! REST read's `403` `OperationOutcome` and serves a `mask`-obligated
//! caller the masked pathway; FHIR search leaves a denied pathway out of
//! the Bundle, so `total` counts only what is shown.
//!
//! Its **own test binary**: `require_auth`/`policy`/`verifier` are
//! process-wide `OnceLock`s, as in `tests/list_search_check_duplicates_authz.rs`.
//! `#[ignore]`d — run with
//! `DB_URL=… cargo test --test fhir_record_level -- --ignored`.

#![cfg(feature = "paseto")]

use axa_test::prelude::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use care_pathway_service::app::App;
use ed25519_dalek::SigningKey;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

const SEED: [u8; 32] = [7; 32];
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

fn pathway(name: &str, setting: &str, provider_name: &str) -> Value {
    json!({
        "name": name,
        "care_setting": setting,
        "provider_id": format!("trust-{}", uuid::Uuid::new_v4().simple()),
        "provider_name": provider_name,
        "pathway_code": format!("FH1-{}", uuid::Uuid::new_v4().simple()),
    })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test fhir_record_level -- --ignored`"]
async fn fhir_read_and_search_take_the_rest_reads_decision() {
    let (keys, kid) = keys_and_kid();
    // `dept=outsider` may not read a sensitive-setting pathway;
    // `dept=masked` reads every pathway under a `mask` obligation.
    let policy = json!({
        "rules": [
            { "effect": "deny", "actions": ["read"],
              "when": { "dept": ["outsider"], "resource.sensitive_setting": ["true"] } },
            { "effect": "allow", "actions": ["read"],
              "when": { "dept": ["masked"] }, "obligations": ["mask"] },
            { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
        ]
    });
    unsafe {
        std::env::set_var("CARE_PATHWAY_REQUIRE_AUTH", "1");
        std::env::set_var("CARE_PATHWAY_PASETO_KEYS", keys.to_string());
        std::env::set_var("CARE_PATHWAY_ABAC_POLICY", policy.to_string());
        std::env::set_var(
            "CARE_PATHWAY_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!(
                "care-pathway-fhir-record-level-index-{}",
                std::process::id()
            )),
        );
        std::env::set_var("CARE_PATHWAY_SEARCH_BOOT_REINDEX", "0");
    }

    request::<App, _, _>(|request, _ctx| async move {
        let tag = uuid::Uuid::new_v4().simple().to_string();
        let provider = format!("RiversideTrust{tag}");
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let mut pids = Vec::new();
        for body in [
            pathway(&format!("Zephyr crisis {tag}"), "MentalHealth", &provider),
            pathway(&format!("Zephyr hip {tag}"), "Inpatient", &provider),
        ] {
            let created = request
                .post("/api/care-pathways")
                .add_header(wk.clone(), wv.clone())
                .json(&body)
                .await;
            assert_eq!(created.status_code(), 200, "{}", created.text());
            pids.push(created.json::<Value>()["pid"].as_str().unwrap().to_string());
        }
        let (sensitive, ordinary) = (pids[0].clone(), pids[1].clone());
        let (out_k, out_v) = auth_header(&mint(&kid, &[("dept", &["outsider"])]));
        let (mask_k, mask_v) = auth_header(&mint(&kid, &[("dept", &["masked"])]));

        // Control: the REST read is denied.
        let rest = request
            .get(&format!("/api/care-pathways/{sensitive}"))
            .add_header(out_k.clone(), out_v.clone())
            .await;
        assert_eq!(rest.status_code(), 403);

        // FHIR read: the same 403, as an OperationOutcome.
        let read = request
            .get(&format!("/fhir/PlanDefinition/{sensitive}"))
            .add_header(out_k.clone(), out_v.clone())
            .await;
        assert_eq!(read.status_code(), 403, "{}", read.text());
        let outcome: Value = read.json();
        assert_eq!(outcome["resourceType"], "OperationOutcome");
        assert_eq!(outcome["issue"][0]["code"], "forbidden");

        // FHIR read under a mask obligation: the provider is masked.
        let masked: Value = request
            .get(&format!("/fhir/PlanDefinition/{ordinary}"))
            .add_header(mask_k.clone(), mask_v.clone())
            .await
            .json();
        let shown = masked.to_string();
        assert!(
            !shown.contains(&provider),
            "provider shown whole under mask: {shown}"
        );

        // FHIR search: the denied pathway is left out, and `total` counts
        // only what is shown.
        let bundle: Value = request
            .get(&format!("/fhir/PlanDefinition?name={tag}"))
            .add_header(out_k.clone(), out_v.clone())
            .await
            .json();
        let ids: Vec<&str> = bundle["entry"]
            .as_array()
            .map(|e| {
                e.iter()
                    .filter_map(|x| x["resource"]["id"].as_str())
                    .collect()
            })
            .unwrap_or_default();
        assert!(
            !ids.contains(&sensitive.as_str()),
            "denied pathway in the Bundle: {bundle}"
        );
        assert!(
            ids.contains(&ordinary.as_str()),
            "allowed pathway missing: {bundle}"
        );
        assert_eq!(bundle["total"].as_u64(), Some(ids.len() as u64), "{bundle}");
    })
    .await;
}
