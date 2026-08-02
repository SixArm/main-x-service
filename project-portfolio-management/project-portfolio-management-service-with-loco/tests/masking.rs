//! End-to-end proof of the privacy layer: the `mask` **obligation** on
//! `GET /api/plans/{pid}`, the always-masked view, and the audited GDPR
//! export.
//!
//! The interesting property is the obligation. With
//! `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` on and a policy that
//! allows read but attaches `"obligations": ["mask"]` for one class of
//! caller, that caller gets the **redacted** record from the ordinary
//! endpoint — there is no second URL to know about, and no way to ask
//! for the unredacted form. The same caller's export is redacted too,
//! and says so in its envelope.
//!
//! Its **own test binary**, distinct from `tests/enforcement.rs`,
//! because `require_auth` / `policy` / `verifier` are process-wide
//! `OnceLock`s: a policy set here would otherwise leak into (or be
//! pre-empted by) that suite. Same pattern as the organization /
//! care-pathway / case services' `tests/masking.rs` (and the lesson
//! learned the hard way in case's — two masking-flavoured tests sharing
//! one binary let whichever boots first silently win the policy for
//! both).
//!
//! `#[ignore]`d — boots the app, so it needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`. Run with
//! `cargo test --test masking -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

const SEED: [u8; 32] = [41; 32];
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

/// A representative plan carrying every field the privacy layer
/// redacts.
fn apollo_plan() -> Value {
    json!({
        "kind": "Project",
        "name": "Apollo platform migration",
        "code": "PROJ-2026",
        "owner_org_id": "organization:9a2f1e2a-0000",
        "owner_org_name": "Acme Astronautics",
        "lead_ref": "person:0c4f1e2a-0000-4000-8000-000000000000",
        "goals": [{ "title": "Cut p95 latency" }]
    })
}

/// The `mask` obligation redacts the ordinary `GET` for the caller it
/// applies to, leaves an ordinary caller's read intact, and carries
/// through to the GDPR export — which declares itself partial.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test masking -- --ignored`"]
async fn mask_obligation_redacts_reads_and_exports() {
    let (keys, kid) = keys_and_kid();
    // `dept=partner` may read, but only masked. Everyone else reads in
    // full by the default policy; `access=write` may create. Set
    // BEFORE the app boots — the auth `OnceLock`s read these on first
    // use.
    let policy = json!({
        "rules": [
            { "effect": "allow", "actions": ["read"],
              "when": { "dept": ["partner"] }, "obligations": ["mask"] },
            { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
        ]
    });
    unsafe {
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH", "1");
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS", keys.to_string());
        std::env::set_var(
            "PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY",
            policy.to_string(),
        );
        // Keep this binary's Tantivy index out of the working directory
        // and its boot rebuild out of the way (see tests/requests/mod.rs).
        std::env::set_var(
            "PROJECT_PORTFOLIO_MANAGEMENT_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!(
                "project-portfolio-management-masking-index-{}",
                std::process::id()
            )),
        );
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_SEARCH_BOOT_REINDEX", "0");
    }

    request::<App, _, _>(|request, _ctx| async move {
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let created = request
            .post("/api/plans")
            .add_header(wk, wv)
            .json(&apollo_plan())
            .await;
        assert_eq!(created.status_code(), 200, "write caller can create");
        let body: Value = serde_json::from_str(&created.text()).unwrap();
        let pid = body["pid"].as_str().expect("pid").to_string();

        // An ordinary caller reads the record in full.
        let (ok_k, ok_v) = auth_header(&mint(&kid, &[]));
        let full: Value = serde_json::from_str(
            &request
                .get(&format!("/api/plans/{pid}"))
                .add_header(ok_k.clone(), ok_v.clone())
                .await
                .text(),
        )
        .unwrap();
        assert_eq!(full["owner_org_id"], "organization:9a2f1e2a-0000");
        assert_eq!(
            full["lead_ref"],
            "person:0c4f1e2a-0000-4000-8000-000000000000"
        );

        // The masked caller reads the SAME url and gets the redacted
        // record — there is no unredacted form to ask for.
        let (m_k, m_v) = auth_header(&mint(&kid, &[("dept", &["partner"])]));
        let masked: Value = serde_json::from_str(
            &request
                .get(&format!("/api/plans/{pid}"))
                .add_header(m_k.clone(), m_v.clone())
                .await
                .text(),
        )
        .unwrap();
        assert!(
            masked["lead_ref"].is_null(),
            "the plan lead must not survive masking"
        );
        assert_ne!(masked["owner_org_id"], "organization:9a2f1e2a-0000");
        assert!(masked["owner_org_id"].as_str().unwrap().contains('*'));
        assert_ne!(masked["owner_org_name"], "Acme Astronautics");
        // The record stays recognisable, or masking has cost more than
        // it bought.
        assert_eq!(masked["name"], "Apollo platform migration");
        assert_eq!(masked["code"], "PROJ-2026");

        // The export follows the same decision, and says which it is.
        let partial: Value = serde_json::from_str(
            &request
                .get(&format!("/api/plans/{pid}/export"))
                .add_header(m_k, m_v)
                .await
                .text(),
        )
        .unwrap();
        assert_eq!(partial["masked"], true);
        assert_eq!(partial["pid"], pid);
        assert!(partial["record"]["lead_ref"].is_null());

        let complete: Value = serde_json::from_str(
            &request
                .get(&format!("/api/plans/{pid}/export"))
                .add_header(ok_k.clone(), ok_v.clone())
                .await
                .text(),
        )
        .unwrap();
        assert_eq!(complete["masked"], false);
        assert_eq!(
            complete["record"]["lead_ref"],
            "person:0c4f1e2a-0000-4000-8000-000000000000"
        );

        // Both exports are on the audit trail: a read of owner/lead
        // information is itself a recordable event.
        let audit: Value = serde_json::from_str(
            &request
                .get(&format!("/api/plans/{pid}/audit"))
                .add_header(ok_k.clone(), ok_v.clone())
                .await
                .text(),
        )
        .unwrap();
        let export_count = audit
            .as_array()
            .map(|rows| rows.iter().filter(|r| r["action"] == "exported").count())
            .unwrap_or_default();
        assert_eq!(export_count, 2, "every export is audited: {audit}");

        // The always-masked view needs no policy to produce the same
        // redaction for an ordinary caller.
        let view: Value = serde_json::from_str(
            &request
                .get(&format!("/api/plans/{pid}/masked"))
                .add_header(ok_k, ok_v)
                .await
                .text(),
        )
        .unwrap();
        assert!(view["lead_ref"].is_null());
        assert_ne!(view["owner_org_id"], "organization:9a2f1e2a-0000");
    })
    .await;

    unsafe {
        std::env::remove_var("PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH");
        std::env::remove_var("PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY");
        std::env::remove_var("PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS");
    }
}
