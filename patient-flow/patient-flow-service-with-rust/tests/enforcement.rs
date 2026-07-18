//! Auth-activation matrix over the live routes (PF-T17, PF-R14).
//!
//! Its own test binary (not part of `tests/mod.rs`) because
//! `PATIENT_FLOW_REQUIRE_AUTH` and the key set are cached in
//! process-wide `OnceLock`s — the flag must be set **before** the app
//! boots, once per process. A throwaway Ed25519 key mints PASETO
//! tokens + the matching key set in-process (no auth service needed).
//!
//! `#[ignore]`d: boots the app (needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`). Run with
//! `cargo test --test enforcement -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use patient_flow_service::app::App;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";
/// Throwaway Ed25519 seed — mints test tokens only, never a secret.
const SEED: [u8; 32] = [7; 32];

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

/// Mint a PASETO `v4.public` with the given ABAC subject attributes.
fn sign_with_attrs(kid: &str, attrs: &[(&str, &[&str])]) -> String {
    let attrs_map: serde_json::Map<String, Value> = attrs
        .iter()
        .map(|(key, values)| ((*key).to_string(), json!(values)))
        .collect();
    let iat: i64 = 1_700_000_000;
    let payload = json!({
        "sub": "11111111-1111-1111-1111-111111111111",
        "email": "nurse@example.com", "name": "Test Nurse",
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

/// The deployment policy for the matrix: machine peers do everything,
/// `access=write` writes, and **any** authenticated read is allowed
/// but carries the `mask` obligation — the corridor-screen posture.
fn test_policy() -> String {
    json!({ "rules": [
        { "effect": "allow",
          "actions": ["read", "write", "delete", "destructive"],
          "when": { "svc": ["true"] } },
        { "effect": "allow", "actions": ["write"], "when": { "access": ["write"] } },
        { "effect": "allow", "actions": ["read"], "when": {}, "obligations": ["mask"] }
    ] })
    .to_string()
}

/// The full activation matrix in one test (one boot ⇒ one set of
/// cached `OnceLock`s): public paths stay open, missing tokens are
/// 401, the ABAC actions gate mutations, and an allow-with-`mask`
/// read returns the **redacted** whiteboard.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_on_gates_and_masks_the_real_stack() {
    let (keys, kid) = keys_and_kid();
    // The auth OnceLocks read these on first use — set before boot.
    // `set_var` is `unsafe` in edition 2024; single-threaded setup step.
    unsafe {
        std::env::set_var("PATIENT_FLOW_REQUIRE_AUTH", "1");
        std::env::set_var("PATIENT_FLOW_PASETO_KEYS", keys.to_string());
        std::env::set_var("PATIENT_FLOW_ABAC_POLICY", test_policy());
    }
    let reader = sign_with_attrs(&kid, &[]);
    let writer = sign_with_attrs(&kid, &[("access", &["write"])]);
    let machine = sign_with_attrs(&kid, &[("svc", &["true"])]);

    request::<App, _, _>(|request, _ctx| async move {
        let bearer = |token: &str| format!("Bearer {token}");

        // Public allow-list stays open without a token.
        assert_eq!(request.get("/_health").await.status_code(), 200);
        assert_eq!(request.get("/metrics.prom").await.status_code(), 200);

        // Protected: no token ⇒ 401; junk token ⇒ 401.
        assert_eq!(request.get("/api/wards").await.status_code(), 401);
        assert_eq!(
            request
                .get("/api/wards")
                .add_header("authorization", "Bearer v4.public.junk")
                .await
                .status_code(),
            401
        );

        // Empty-attrs token: reads allowed (mask rule), mutations 403.
        assert_eq!(
            request
                .get("/api/wards")
                .add_header("authorization", bearer(&reader))
                .await
                .status_code(),
            200
        );
        assert_eq!(
            request
                .post("/api/sites")
                .add_header("authorization", bearer(&reader))
                .json(&json!({ "name": "S" }))
                .await
                .status_code(),
            403
        );

        // `access=write` builds the topology + admits a patient.
        let post = |path: &'static str, body: Value| {
            let request = &request;
            let writer = writer.clone();
            async move {
                let response = request
                    .post(path)
                    .add_header("authorization", format!("Bearer {writer}"))
                    .json(&body)
                    .await;
                assert_eq!(response.status_code(), 200, "{path} as access=write");
                response.json::<Value>()
            }
        };
        let site = post("/api/sites", json!({ "name": "Masked Site" })).await;
        let ward = post(
            "/api/wards",
            json!({ "site_pid": site["pid"], "name": "Masked Ward", "code": "MW", "kind": "inpatient" }),
        )
        .await;
        let bay = post(
            "/api/bays",
            json!({ "ward_pid": ward["pid"], "name": "Bay M", "sex_designation": "flexible" }),
        )
        .await;
        let bed = post(
            "/api/beds",
            json!({ "bay_pid": bay["pid"], "number": "MW-1" }),
        )
        .await;
        let stay_body = json!({
            "person_ref": format!("person:{}", uuid::Uuid::new_v4()),
            "display_name": "Very Identifiable Name",
            "source": "ed", "bed_pid": bed["pid"],
        });
        let stay = request
            .post("/api/stays")
            .add_header("authorization", bearer(&writer))
            .json(&stay_body)
            .await;
        assert_eq!(stay.status_code(), 200);

        // The reader's whiteboard is MASKED: the mask obligation
        // redacts the name; bed state stays visible.
        let board: Value = {
            let response = request
                .get(&format!("/api/whiteboard/{}", ward["pid"].as_str().unwrap()))
                .add_header("authorization", bearer(&reader))
                .await;
            assert_eq!(response.status_code(), 200);
            response.json()
        };
        assert_eq!(board["masked"], true, "mask obligation reaches the board");
        assert_eq!(board["cards"][0]["state"], "occupied");
        assert_eq!(
            board["cards"][0]["display_name"], "\u{2022}\u{2022}\u{2022}",
            "patient name is redacted for the masked reader"
        );
        assert_eq!(board["cards"][0]["alerts"].as_array().map(Vec::len), Some(0));

        // Writer may not delete; the machine peer may.
        let bed_pid = bed["pid"].as_str().unwrap();
        assert_eq!(
            request
                .delete(&format!("/api/beds/{bed_pid}"))
                .add_header("authorization", bearer(&writer))
                .await
                .status_code(),
            403,
            "write is not delete"
        );
        assert_eq!(
            request
                .delete(&format!("/api/beds/{bed_pid}"))
                .add_header("authorization", bearer(&machine))
                .await
                .status_code(),
            200,
            "svc=true may delete"
        );
    })
    .await;
}
