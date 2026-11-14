//! Record-level decisions on the **instance layer** (spec §13 T-15h).
//!
//! An enrolment names a person and carries their scores, so it takes the
//! decision its pathway template takes: a denied caller gets `404` on the
//! detail, list, cohort, outcomes, segments and time-analysis, a `mask`
//! obligation withholds `subject_ref` and mental-health measures, and the
//! cross-pathway views leave a denied pathway out.
//!
//! Its **own test binary**, because `require_auth` / `policy` / `verifier`
//! are process-wide `OnceLock`s (see `tests/masking.rs`).
//!
//! `#[ignore]`d — needs PostgreSQL. Run with
//! `cargo test --test instance_authz -- --ignored`.

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

/// Fixed signing seed, so the published key set and the minted tokens
/// agree without a running auth service.
const SEED: [u8; 32] = [7; 32];
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

/// Mint a `v4.public` token carrying the given ABAC subject attributes.
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

fn pathway(name: &str, code: &str, icd10: &str) -> Value {
    json!({
        "name": name,
        "care_setting": "Inpatient",
        "provider_id": "trust-9",
        "provider_name": "Riverside Trust",
        "pathway_code": code,
        "condition_codes": [{"system": "Icd10", "code": icd10}],
        "identifiers": []
    })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test instance_authz -- --ignored`"]
async fn instances_take_the_record_level_decision_of_their_pathway() {
    let (keys, kid) = keys_and_kid();
    let policy = json!({
        "rules": [
            { "effect": "deny", "actions": ["read", "write"],
              "when": { "dept": ["outsider"], "resource.sensitive_setting": ["true"] } },
            { "effect": "allow", "actions": ["read", "write"],
              "when": { "dept": ["partner"], "resource.sensitive_setting": ["true"] },
              "obligations": ["mask"] },
            { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
        ]
    });
    unsafe {
        std::env::set_var("CARE_PATHWAY_REQUIRE_AUTH", "1");
        std::env::set_var("CARE_PATHWAY_PASETO_KEYS", keys.to_string());
        std::env::set_var("CARE_PATHWAY_ABAC_POLICY", policy.to_string());
        std::env::set_var("CARE_PATHWAY_FLOW_METRICS_MIN_COHORT", "1");
        std::env::set_var(
            "CARE_PATHWAY_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!(
                "care-pathway-instance-authz-{}",
                std::process::id()
            )),
        );
        std::env::set_var("CARE_PATHWAY_SEARCH_BOOT_REINDEX", "0");
    }

    request::<App, _, _>(|request, ctx| async move {
        let writer = auth_header(&mint(&kid, &[("access", &["write"])]));
        let outsider = auth_header(&mint(
            &kid,
            &[("dept", &["outsider"]), ("access", &["write"])],
        ));
        let partner = auth_header(&mint(
            &kid,
            &[("dept", &["partner"]), ("access", &["write"])],
        ));

        // One sensitive pathway (ICD-10 F43.1) and one orthopaedic one.
        let mut pids = Vec::new();
        for (name, code, icd) in [
            ("ICU psychology", "ICU-PSY-01", "F43.1"),
            ("Hip fracture", "ORTHO-01", "M16.1"),
        ] {
            let made = request
                .post("/api/care-pathways")
                .add_header(writer.0.clone(), writer.1.clone())
                .json(&pathway(name, code, icd))
                .await;
            assert_eq!(made.status_code(), 200, "{}", made.text());
            pids.push(made.json::<Value>()["pid"].as_str().unwrap().to_string());
        }
        let (sensitive, ortho) = (pids[0].clone(), pids[1].clone());

        // Enrol one person on each; record a mental-health score on the
        // sensitive one.
        let mut instances = Vec::new();
        for pathway_pid in [&sensitive, &ortho] {
            let enrolled = request
                .post(&format!("/api/care-pathways/{pathway_pid}/instances"))
                .add_header(writer.0.clone(), writer.1.clone())
                .json(&json!({ "subject_ref": "person:22222222-2222-2222-2222-222222222222" }))
                .await;
            assert_eq!(enrolled.status_code(), 200, "{}", enrolled.text());
            instances.push(
                enrolled.json::<Value>()["pid"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            );
        }
        let (s_inst, o_inst) = (instances[0].clone(), instances[1].clone());
        let scored = request
            .post(&format!("/api/instances/{s_inst}/measures"))
            .add_header(writer.0.clone(), writer.1.clone())
            .json(&json!({ "instrument": "pcl5", "value_numeric": 40 }))
            .await;
        assert_eq!(scored.status_code(), 200, "{}", scored.text());

        // The outsider: 404 on every instance-layer read of the sensitive
        // pathway, 200 on the orthopaedic one.
        for path in [
            format!("/api/instances/{s_inst}"),
            format!("/api/instances/{s_inst}/segments"),
            format!("/api/instances/{s_inst}/time-analysis"),
            format!("/api/instances/{s_inst}/timeline"),
            format!("/api/care-pathways/{sensitive}/instances"),
            format!("/api/care-pathways/{sensitive}/cohort"),
            format!("/api/care-pathways/{sensitive}/outcomes"),
            format!("/api/care-pathways/{sensitive}/time-analysis"),
            format!("/api/care-pathways/{sensitive}/constraints"),
        ] {
            let denied = request
                .get(&path)
                .add_header(outsider.0.clone(), outsider.1.clone())
                .await;
            assert_eq!(denied.status_code(), 404, "{path}: {}", denied.text());
        }
        for path in [
            format!("/api/instances/{o_inst}"),
            format!("/api/care-pathways/{ortho}/instances"),
            format!("/api/care-pathways/{ortho}/cohort"),
            format!("/api/care-pathways/{ortho}/outcomes"),
        ] {
            let ok = request
                .get(&path)
                .add_header(outsider.0.clone(), outsider.1.clone())
                .await;
            assert_eq!(ok.status_code(), 200, "{path}: {}", ok.text());
        }
        // Screening reach is an aggregate of the same scores: denied and
        // masked callers get 404, never the figures.
        for who in [&outsider, &partner] {
            let r = request
                .get(&format!(
                    "/api/care-pathways/{sensitive}/screening?instrument=pcl5"
                ))
                .add_header(who.0.clone(), who.1.clone())
                .await;
            assert_eq!(r.status_code(), 404, "{}", r.text());
        }
        let r = request
            .get(&format!(
                "/api/care-pathways/{sensitive}/screening?instrument=pcl5"
            ))
            .add_header(writer.0.clone(), writer.1.clone())
            .await;
        assert_eq!(r.status_code(), 200, "{}", r.text());
        // A denied write is also 404.
        let write = request
            .post(&format!("/api/instances/{s_inst}/events"))
            .add_header(outsider.0.clone(), outsider.1.clone())
            .json(&json!({ "kind": "note" }))
            .await;
        assert_eq!(write.status_code(), 404, "{}", write.text());

        // No credential is still 401, not 404.
        let anon = request.get(&format!("/api/instances/{s_inst}")).await;
        assert_eq!(anon.status_code(), 401);

        // Cross-pathway views leave the denied pathway out.
        let writer_load: Value = request
            .get("/api/instances/caseload")
            .add_header(writer.0.clone(), writer.1.clone())
            .await
            .json();
        let outsider_load: Value = request
            .get("/api/instances/caseload")
            .add_header(outsider.0.clone(), outsider.1.clone())
            .await
            .json();
        assert_eq!(writer_load["open"], 2, "{writer_load}");
        assert_eq!(outsider_load["open"], 1, "{outsider_load}");

        // The masked caller reads, with the person and the score withheld.
        let masked: Value = request
            .get(&format!("/api/instances/{s_inst}"))
            .add_header(partner.0.clone(), partner.1.clone())
            .await
            .json();
        assert_eq!(masked["masked"], true, "{masked}");
        assert!(masked["instance"]["subject_ref"].is_null(), "{masked}");
        assert_eq!(masked["measures"].as_array().unwrap().len(), 0, "{masked}");
        let listed: Value = request
            .get(&format!("/api/care-pathways/{sensitive}/instances"))
            .add_header(partner.0.clone(), partner.1.clone())
            .await
            .json();
        assert!(listed[0]["subject_ref"].is_null(), "{listed}");
        // ...and may not write.
        let masked_write = request
            .post(&format!("/api/instances/{s_inst}/events"))
            .add_header(partner.0.clone(), partner.1.clone())
            .json(&json!({ "kind": "note" }))
            .await;
        assert_eq!(masked_write.status_code(), 404);
        // The unmasked caller sees both.
        let full: Value = request
            .get(&format!("/api/instances/{s_inst}"))
            .add_header(writer.0.clone(), writer.1.clone())
            .await
            .json();
        assert_eq!(
            full["instance"]["subject_ref"],
            "person:22222222-2222-2222-2222-222222222222"
        );
        assert_eq!(full["measures"].as_array().unwrap().len(), 1, "{full}");

        // The public flow gauges never label the sensitive pathway.
        let set = care_pathway_service::flow_metrics::refresh_once(&ctx)
            .await
            .expect("refresh");
        assert!(
            set.rows.iter().all(|r| r.pathway_pid != sensitive),
            "a special-category pathway must not be labelled on /metrics.prom"
        );
        assert!(set.rows.iter().any(|r| r.pathway_pid == ortho));
        assert!(set.suppressed_pathways >= 1);
    })
    .await;

    unsafe {
        std::env::remove_var("CARE_PATHWAY_REQUIRE_AUTH");
        std::env::remove_var("CARE_PATHWAY_ABAC_POLICY");
        std::env::remove_var("CARE_PATHWAY_PASETO_KEYS");
        std::env::remove_var("CARE_PATHWAY_FLOW_METRICS_MIN_COHORT");
    }
}
