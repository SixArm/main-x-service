//! Auth-activation persona matrix over the live routes (CRM-R15).
//!
//! Its own test binary (not part of `tests/mod.rs`) because
//! `CRM_REQUIRE_AUTH` and the key set are cached in process-wide
//! `OnceLock`s — the flag must be set **before** the app boots, once
//! per process. A throwaway Ed25519 key mints PASETO tokens + the
//! matching key set in-process (no auth service needed).
//!
//! `#[ignore]`d: boots the app (needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`). Run with
//! `cargo test --test enforcement -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use contact_relationship_management_service::app::App;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";
/// Throwaway Ed25519 seed — mints test tokens only, never a secret.
const SEED: [u8; 32] = [5; 32];

/// The published-key-set JSON + `kid` for the test key.
fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// Mint a PASETO `v4.public` with a given `sub` and ABAC attributes.
fn sign_as(kid: &str, sub: &str, attrs: &[(&str, &[&str])]) -> String {
    let attrs_map: serde_json::Map<String, Value> = attrs
        .iter()
        .map(|(key, values)| ((*key).to_string(), json!(values)))
        .collect();
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": sub,
        "email": "rep@example.com", "name": "Test Rep",
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

/// The deployment policy (spec `auth.md` personas): machine peers do
/// everything; `access=write` writes; the record's own owner (`$sub`
/// matching `resource.owner`, CRM-T24) reads unmasked; any other
/// authenticated read is allowed but carries the `mask` obligation —
/// first-match-wins, so an unowned record (no `resource.owner` at
/// all) falls through the owner rule to this masked catch-all.
fn test_policy() -> String {
    json!({ "rules": [
        { "effect": "allow",
          "actions": ["read", "write", "delete", "destructive"],
          "when": { "svc": ["true"] } },
        { "effect": "allow", "actions": ["read"], "when": { "resource.owner": ["$sub"] } },
        { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } },
        { "effect": "allow", "actions": ["read"], "when": {}, "obligations": ["mask"] }
    ] })
    .to_string()
}

/// The activation matrix in one boot: public paths stay open, missing
/// tokens are 401, ABAC gates mutations, and the writer persona runs
/// a real consent-gated flow end-to-end under enforcement.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_gates_the_real_stack() {
    let (keys, kid) = keys_and_kid();
    // `set_var` is `unsafe` in edition 2024; single-threaded setup step.
    unsafe {
        std::env::set_var("CRM_REQUIRE_AUTH", "1");
        std::env::set_var("CRM_PASETO_KEYS", keys.to_string());
        std::env::set_var("CRM_ABAC_POLICY", test_policy());
    }
    let reader = sign_as(&kid, "reader-user", &[]);
    let writer = sign_as(&kid, "writer-user", &[("access", &["write"])]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");

        // Public allow-list stays open without a token.
        assert_eq!(request.get("/_health").await.status_code(), 200);
        assert_eq!(request.get("/metrics.prom").await.status_code(), 200);

        // Protected: no token ⇒ 401; junk ⇒ 401.
        assert_eq!(request.get("/api/contacts").await.status_code(), 401);
        assert_eq!(
            request
                .get("/api/contacts")
                .add_header("authorization", "Bearer v4.public.junk")
                .await
                .status_code(),
            401
        );

        // Reader: GET allowed, POST 403.
        assert_eq!(
            request
                .get("/api/contacts")
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .post("/api/contacts")
                .add_header("authorization", bearer(&reader))
                .json(&json!({ "person_ref": "person:x" }))
                .await
                .status_code(),
            403
        );

        // Writer: creates a contact, grants consent, reads history.
        let contact = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                "display_name": "Enforced Contact",
            }))
            .await;
        assert_eq!(contact.status_code(), 200);
        let contact_pid = contact.json::<Value>()["pid"].as_str().unwrap().to_string();
        assert_eq!(
            request
                .post(&format!("/api/contacts/{contact_pid}/consent"))
                .add_header("authorization", bearer(&writer))
                .json(&json!({ "action": "granted", "source": "form" }))
                .await
                .status_code(),
            200
        );
        let history = request
            .get(&format!("/api/contacts/{contact_pid}/consent"))
            .add_header("authorization", bearer(&reader))
            .await;
        assert_eq!(history.status_code(), 200);
        assert_eq!(history.json::<Value>().as_array().unwrap().len(), 1);

        // CRM-T24: record-level ABAC on the contact/deal read paths.
        // The deal/contact's own owner reads unmasked; a stranger
        // (the `reader` persona above, whose `sub` matches no
        // `resource.owner`) reads the same records masked — `reader`
        // itself proves the "GET allowed" case is unaffected, and the
        // sub-below (`owner-user`) proves ownership actually changes
        // the outcome, not just the default policy shape.
        let owner_sub = uuid::Uuid::new_v4();
        let owner = sign_as(&kid, &owner_sub.to_string(), &[]);
        let owner_ref = format!("worker:{owner_sub}");

        // A contact owned by `owner_sub`: the owner sees the real
        // `preferred_channel`; the stranger sees it masked.
        let owned_contact = request
            .post("/api/contacts")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
                "display_name": "Owned Contact",
                "owner_ref": owner_ref,
                "preferred_channel": "phone",
            }))
            .await;
        assert_eq!(owned_contact.status_code(), 200);
        let owned_contact_pid = owned_contact.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();

        let as_owner = request
            .get(&format!("/api/contacts/{owned_contact_pid}"))
            .add_header("authorization", bearer(&owner))
            .await;
        assert_eq!(as_owner.status_code(), 200);
        assert_eq!(
            as_owner.json::<Value>()["contact"]["preferred_channel"],
            "phone"
        );

        let as_stranger = request
            .get(&format!("/api/contacts/{owned_contact_pid}"))
            .add_header("authorization", bearer(&reader))
            .await;
        assert_eq!(as_stranger.status_code(), 200);
        assert_eq!(
            as_stranger.json::<Value>()["contact"]["preferred_channel"],
            contact_relationship_management_service::auth::MASKED
        );

        // A deal owned by `owner_sub`: the owner's list view sees the
        // real `amount_minor`; the stranger's sees it nulled.
        let pipeline = request
            .post("/api/pipelines")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "name": "Enforced Pipeline",
                "stages": [
                    { "name": "Open", "probability_percent": 50 },
                    { "name": "Won", "probability_percent": 100, "is_won": true },
                ],
            }))
            .await;
        assert_eq!(pipeline.status_code(), 200);
        let pipeline_pid = pipeline.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();

        let owned_deal = request
            .post("/api/deals")
            .add_header("authorization", bearer(&writer))
            .json(&json!({
                "name": "Owned Deal",
                "pipeline_pid": pipeline_pid,
                "amount_minor": 500_000,
                "currency": "GBP",
                "owner_ref": owner_ref,
            }))
            .await;
        assert_eq!(owned_deal.status_code(), 200);
        let owned_deal_pid = owned_deal.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();

        let find_amount = |rows: &Value| -> Value {
            rows.as_array()
                .unwrap()
                .iter()
                .find(|row| row["pid"] == owned_deal_pid)
                .expect("the owned deal is in the list")["amount_minor"]
                .clone()
        };

        let deals_as_owner = request
            .get(&format!("/api/deals?pipeline={pipeline_pid}"))
            .add_header("authorization", bearer(&owner))
            .await;
        assert_eq!(deals_as_owner.status_code(), 200);
        assert_eq!(find_amount(&deals_as_owner.json::<Value>()), json!(500_000));

        let deals_as_stranger = request
            .get(&format!("/api/deals?pipeline={pipeline_pid}"))
            .add_header("authorization", bearer(&reader))
            .await;
        assert_eq!(deals_as_stranger.status_code(), 200);
        assert!(find_amount(&deals_as_stranger.json::<Value>()).is_null());
    })
    .await;
}
