//! spec §13 T-9: a policy denial on the single-case `subject_of` link
//! endpoints (`/api/cases/{pid}/links*`) must be reported as `404`, not
//! `403` — matching the family precedent care-pathway's `continues_as`
//! established (`agents/share/cross-service-linking.md` §10.2). A `403`
//! would disclose that *this case has a `subject_of` edge*, which is
//! itself sensitive; an empty edge list and a denied read must be
//! indistinguishable. This is pinned DB-free in
//! `src/controllers/links.rs`'s unit tests; this file proves it holds
//! over the real HTTP surface, and contrasts it with the case *record*
//! endpoint, which deliberately keeps `403` (a case is not disclosed
//! merely by existing).
//!
//! Its **own test binary**, mirroring `tests/masking.rs`'s reasoning: the
//! process-wide `require_auth`/`policy`/`verifier` `OnceLock`s must carry
//! this file's policy without racing another suite's.
//!
//! `#[ignore]`d: boots the app (needs PostgreSQL via `config/test.yaml` /
//! `DATABASE_URL`). Run with `cargo test --test links_masking -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use case_service::app::App;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

const SEED: [u8; 32] = [11; 32];
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
        "sub": "22222222-2222-2222-2222-222222222222",
        "email": "bob@example.com", "name": "Bob",
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

/// spec §13 T-9: a `dept=blocked` caller (denied `read` on investigation
/// cases by policy) gets `404` — not `403` — reading a case's
/// `subject_of` links, indistinguishable from a case with no edges. An
/// ordinary caller sees the same link on the same endpoint. The case's
/// own native `GET /{pid}` stays `403` for the blocked caller, by
/// contrast — the link endpoints' `404` fold is deliberately narrower.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test links_masking -- --ignored`"]
async fn a_denied_links_read_is_404_not_403() {
    let (keys, kid) = keys_and_kid();
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
        // Create a case and a subject_of link with a write-capable token.
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let created = request
            .post("/api/cases")
            .add_header(wk.clone(), wv.clone())
            .json(
                &json!({ "title": "Sensitive investigation", "agency_id": "dwp",
                            "case_type": "Investigation" }),
            )
            .await;
        assert_eq!(created.status_code(), 200, "write caller can create");
        let pid = created.json::<Value>()["pid"]
            .as_str()
            .expect("pid")
            .to_string();

        let person = "person:0c4f1e2a-0000-4000-8000-000000000000";
        let linked = request
            .post(&format!("/api/cases/{pid}/links"))
            .add_header(wk, wv)
            .json(&json!({ "kind": "subject_of", "to_ref": person }))
            .await;
        assert_eq!(linked.status_code(), 200, "write caller can link");

        // An ordinary caller (no dept) reads the link by default.
        let (ok_k, ok_v) = auth_header(&mint(&kid, &[]));
        let ok_links = request
            .get(&format!("/api/cases/{pid}/links"))
            .add_header(ok_k, ok_v)
            .await;
        assert_eq!(ok_links.status_code(), 200, "allowed caller: links ok");
        let rows: Value = serde_json::from_str(&ok_links.text()).unwrap();
        assert_eq!(rows.as_array().unwrap().len(), 1, "the link is visible");

        // A blocked caller (dept=blocked) is denied read on this case.
        let (bk, bv) = auth_header(&mint(&kid, &[("dept", &["blocked"])]));
        assert_eq!(
            request
                .get(&format!("/api/cases/{pid}/links"))
                .add_header(bk.clone(), bv.clone())
                .await
                .status_code(),
            404,
            "spec §13 T-9: a denied links read must be 404, not 403"
        );
        // Contrast: the case's own native record endpoint stays 403 —
        // the T-9 fold is deliberately narrower than the whole case.
        assert_eq!(
            request
                .get(&format!("/api/cases/{pid}"))
                .add_header(bk, bv)
                .await
                .status_code(),
            403,
            "the case record endpoint deliberately keeps 403"
        );
    })
    .await;
}
