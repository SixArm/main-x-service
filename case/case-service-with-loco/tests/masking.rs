//! End-to-end **masking / concealment on every read path** proof (SEC-G2 /
//! SEC-G3). With `CASE_REQUIRE_AUTH` on and an ABAC policy that denies
//! `dept=blocked` callers **read on investigation cases specifically**
//! (a `resource.case_type`-scoped rule), a blocked caller must not be
//! able to discover such a case through **any** read surface — the
//! native `list` and `GET /{pid}`, and the FHIR `read` — while an
//! ordinary caller still sees it. This pins that record-level
//! authorization is applied consistently, not just on the single
//! native GET the audit found guarded.
//!
//! The deny rule is deliberately **resource-scoped**: a subject-only
//! deny (`dept=blocked` alone) is enforced by the **coarse blanket
//! guard** and 403s the whole surface before any record loads — which
//! is correct defense-in-depth but exercises the guard, not SEC-G3.
//! Concealment is the property of callers who pass the guard and are
//! denied on *specific records*; per SEC-V2, a non-negated
//! `resource.*` condition never matches on the coarse (no-record)
//! path, so the guard admits the caller and each read path must do
//! its own record-level check. (The original version of this test
//! used a subject-only deny and therefore never got past the guard —
//! fixed 2026-07-18, QA-CASE-MASK.)
//!
//! Its **own test binary** so the process-wide `require_auth` / `policy` /
//! `verifier` `OnceLock`s carry this test's deny-read policy without
//! polluting the other suites.
//!
//! `#[ignore]`d: boots the app (needs PostgreSQL via `config/test.yaml` /
//! `DATABASE_URL`). Run with `cargo test --test masking -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use case_service::app::App;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
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

/// SEC-G2/G3: a `dept=blocked` caller (denied `read` by policy) must not
/// discover a case via `list`, native `GET /{pid}`, or FHIR `read`, while an
/// ordinary caller (default allow-read) sees it on every path.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test masking -- --ignored`"]
async fn read_is_concealed_on_every_path_for_a_denied_caller() {
    let (keys, kid) = keys_and_kid();
    // Deny `read` on **investigation cases** to callers with
    // `dept=blocked` (resource-scoped ⇒ the coarse guard admits the
    // caller and each read path's record-level check must conceal);
    // everyone else reads by default, and `access=write` may create.
    // Set BEFORE the app boots (the auth OnceLocks read these on
    // first use).
    let policy = json!({
        "rules": [
            { "effect": "deny",  "actions": ["read"],
              "when": { "dept": ["blocked"], "resource.case_type": ["investigation"] } },
            { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
        ]
    });
    unsafe {
        std::env::set_var("CASE_REQUIRE_AUTH", "1");
        std::env::set_var("CASE_PASETO_KEYS", keys.to_string());
        std::env::set_var("CASE_ABAC_POLICY", policy.to_string());
    }

    request::<App, _, _>(|request, _ctx| async move {
        // Create a case with a write-capable token.
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let created = request
            .post("/api/cases")
            .add_header(wk, wv)
            .json(&json!({ "title": "Sensitive investigation", "agency_id": "dwp",
                            "case_type": "Investigation" }))
            .await;
        assert_eq!(created.status_code(), 200, "write caller can create");
        let body: Value = serde_json::from_str(&created.text()).unwrap();
        let pid = body["pid"].as_str().expect("pid").to_string();

        // An ordinary caller (no dept) reads by default — sees it everywhere.
        let (ok_k, ok_v) = auth_header(&mint(&kid, &[]));
        let list = request
            .get("/api/cases")
            .add_header(ok_k.clone(), ok_v.clone())
            .await;
        let arr: Value = serde_json::from_str(&list.text()).unwrap();
        assert!(
            arr.as_array().unwrap().iter().any(|c| c["pid"] == pid),
            "an allowed caller sees the case in the list"
        );
        assert_eq!(
            request
                .get(&format!("/api/cases/{pid}"))
                .add_header(ok_k.clone(), ok_v.clone())
                .await
                .status_code(),
            200,
            "allowed caller: native GET ok"
        );
        assert_eq!(
            request
                .get(&format!("/fhir/Task/{pid}"))
                .add_header(ok_k, ok_v)
                .await
                .status_code(),
            200,
            "allowed caller: FHIR read ok"
        );

        // A blocked caller (dept=blocked) is denied read on EVERY path.
        let (bk, bv) = auth_header(&mint(&kid, &[("dept", &["blocked"])]));
        let blist = request
            .get("/api/cases")
            .add_header(bk.clone(), bv.clone())
            .await;
        let barr: Value = serde_json::from_str(&blist.text()).unwrap();
        assert!(
            barr.as_array().unwrap().iter().all(|c| c["pid"] != pid),
            "SEC-G3: the list must not leak the concealed case to a denied caller"
        );
        assert_eq!(
            request
                .get(&format!("/api/cases/{pid}"))
                .add_header(bk.clone(), bv.clone())
                .await
                .status_code(),
            403,
            "native GET is 403 for the denied caller"
        );
        assert_eq!(
            request
                .get(&format!("/fhir/Task/{pid}"))
                .add_header(bk, bv)
                .await
                .status_code(),
            403,
            "SEC-G2: FHIR read is 403 for the denied caller (was unguarded)"
        );
    })
    .await;
}
