//! End-to-end proof of the privacy layer: the `mask` **obligation** on
//! `GET /api/care-pathways/{pid}`, the always-masked view, and the
//! audited GDPR export.
//!
//! The interesting property is the obligation. With
//! `CARE_PATHWAY_REQUIRE_AUTH` on and a policy that allows read but
//! attaches `"obligations": ["mask"]` for one class of caller, that
//! caller gets the **redacted** record from the ordinary endpoint —
//! there is no second URL to know about, and no way to ask for the
//! unredacted form. The same caller's export is redacted too, and says
//! so in its envelope.
//!
//! Its **own test binary**, because `require_auth` / `policy` /
//! `verifier` are process-wide `OnceLock`s: a policy set here would
//! otherwise leak into (or be pre-empted by) the other suites. Same
//! pattern as the organization service's `tests/masking.rs`.
//!
//! `#[ignore]`d — boots the app, so it needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`. Run with
//! `cargo test --test masking -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use care_pathway_service::app::App;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
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

/// A representative, sensitive-setting pathway carrying every field the
/// privacy layer redacts.
fn stroke_pathway() -> Value {
    json!({
        "name": "Community Mental Health Crisis Pathway",
        "care_setting": "MentalHealth",
        "provider_id": "trust-9",
        "provider_name": "Riverside Mental Health Trust",
        "pathway_code": "MH-CRISIS-01",
        "condition_codes": [{"system": "Icd10", "code": "F32"}],
        "identifiers": [{"scheme": "GuidelineId", "value": "NICE-NG225"}]
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
    // `dept=partner` may read, but only masked when the pathway is a
    // sensitive setting. Everyone else reads in full by the default
    // policy; `access=write` may create. Set BEFORE the app boots — the
    // auth `OnceLock`s read these on first use.
    let policy = json!({
        "rules": [
            { "effect": "allow", "actions": ["read"],
              "when": { "dept": ["partner"], "resource.sensitive_setting": ["true"] },
              "obligations": ["mask"] },
            { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } }
        ]
    });
    unsafe {
        std::env::set_var("CARE_PATHWAY_REQUIRE_AUTH", "1");
        std::env::set_var("CARE_PATHWAY_PASETO_KEYS", keys.to_string());
        std::env::set_var("CARE_PATHWAY_ABAC_POLICY", policy.to_string());
        // Keep this binary's Tantivy index out of the working directory
        // and its boot rebuild out of the way (see tests/requests/mod.rs).
        std::env::set_var(
            "CARE_PATHWAY_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!("care-pathway-masking-index-{}", std::process::id())),
        );
        std::env::set_var("CARE_PATHWAY_SEARCH_BOOT_REINDEX", "0");
    }

    request::<App, _, _>(|request, _ctx| async move {
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let created = request
            .post("/api/care-pathways")
            .add_header(wk, wv)
            .json(&stroke_pathway())
            .await;
        assert_eq!(created.status_code(), 200, "write caller can create");
        let body: Value = serde_json::from_str(&created.text()).unwrap();
        let pid = body["pid"].as_str().expect("pid").to_string();

        // An ordinary caller reads the record in full.
        let (ok_k, ok_v) = auth_header(&mint(&kid, &[]));
        let full: Value = serde_json::from_str(
            &request
                .get(&format!("/api/care-pathways/{pid}"))
                .add_header(ok_k.clone(), ok_v.clone())
                .await
                .text(),
        )
        .unwrap();
        assert_eq!(full["provider_id"], "trust-9");
        assert_eq!(full["provider_name"], "Riverside Mental Health Trust");

        // The masked caller reads the SAME url and gets the redacted
        // record — there is no unredacted form to ask for.
        let (m_k, m_v) = auth_header(&mint(&kid, &[("dept", &["partner"])]));
        let masked: Value = serde_json::from_str(
            &request
                .get(&format!("/api/care-pathways/{pid}"))
                .add_header(m_k.clone(), m_v.clone())
                .await
                .text(),
        )
        .unwrap();
        assert_ne!(masked["provider_id"], "trust-9");
        assert!(masked["provider_id"].as_str().unwrap().contains('*'));
        assert_ne!(masked["provider_name"], "Riverside Mental Health Trust");
        // The record stays recognisable and clinically usable, or
        // masking has cost more than it bought.
        assert_eq!(masked["name"], "Community Mental Health Crisis Pathway");
        assert_eq!(masked["condition_codes"][0]["code"], "F32");

        // The export follows the same decision, and says which it is.
        let partial: Value = serde_json::from_str(
            &request
                .get(&format!("/api/care-pathways/{pid}/export"))
                .add_header(m_k, m_v)
                .await
                .text(),
        )
        .unwrap();
        assert_eq!(partial["masked"], true);
        assert_eq!(partial["pid"], pid);
        assert_ne!(partial["record"]["provider_id"], "trust-9");

        let complete: Value = serde_json::from_str(
            &request
                .get(&format!("/api/care-pathways/{pid}/export"))
                .add_header(ok_k.clone(), ok_v.clone())
                .await
                .text(),
        )
        .unwrap();
        assert_eq!(complete["masked"], false);
        assert_eq!(complete["record"]["provider_id"], "trust-9");

        // The always-masked view needs no policy to produce the same
        // redaction for an ordinary caller.
        let view: Value = serde_json::from_str(
            &request
                .get(&format!("/api/care-pathways/{pid}/masked"))
                .add_header(ok_k, ok_v)
                .await
                .text(),
        )
        .unwrap();
        assert_ne!(view["provider_id"], "trust-9");
    })
    .await;

    unsafe {
        std::env::remove_var("CARE_PATHWAY_REQUIRE_AUTH");
        std::env::remove_var("CARE_PATHWAY_ABAC_POLICY");
        std::env::remove_var("CARE_PATHWAY_PASETO_KEYS");
    }
}

/// A caller reading a **non-sensitive** setting is not masked even by
/// the same `dept=partner` policy — the obligation is conditioned on
/// `resource.sensitive_setting`, which the policy sees only after the
/// record is loaded. This is the record-level decision actually looking
/// at the record, not just the caller.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test masking -- --ignored`"]
async fn non_sensitive_settings_are_not_masked_by_the_same_policy() {
    let (keys, kid) = keys_and_kid();
    let policy = json!({
        "rules": [
            { "effect": "allow", "actions": ["read"],
              "when": { "dept": ["partner"], "resource.sensitive_setting": ["true"] },
              "obligations": ["mask"] },
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
                "care-pathway-masking-index-nonsensitive-{}",
                std::process::id()
            )),
        );
        std::env::set_var("CARE_PATHWAY_SEARCH_BOOT_REINDEX", "0");
    }

    request::<App, _, _>(|request, _ctx| async move {
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let mut ortho = stroke_pathway();
        ortho["name"] = json!("Elective Hip Replacement Pathway");
        ortho["care_setting"] = json!("Inpatient");
        let created = request
            .post("/api/care-pathways")
            .add_header(wk, wv)
            .json(&ortho)
            .await;
        assert_eq!(created.status_code(), 200);
        let pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();

        let (m_k, m_v) = auth_header(&mint(&kid, &[("dept", &["partner"])]));
        let response = request
            .get(&format!("/api/care-pathways/{pid}"))
            .add_header(m_k, m_v)
            .await;
        // No rule matches `dept=partner` against a non-sensitive-setting
        // pathway (the only allow rule requires `resource.sensitive_setting
        // = true`), so the **default** decision applies
        // (`authorization-attributes.md` §5: read ⇒ allow). The read
        // succeeds, unmasked — the obligation only exists where a rule
        // actually granted it.
        assert_eq!(response.status_code(), 200, "default read is allowed");
        let body: Value = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(
            body["provider_id"], "trust-9",
            "no matching rule ⇒ no obligation ⇒ unmasked: {body}"
        );
    })
    .await;

    unsafe {
        std::env::remove_var("CARE_PATHWAY_REQUIRE_AUTH");
        std::env::remove_var("CARE_PATHWAY_ABAC_POLICY");
        std::env::remove_var("CARE_PATHWAY_PASETO_KEYS");
    }
}
