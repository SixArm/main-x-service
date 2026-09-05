//! DB-gated proof of the T-36 operator control-plane endpoint: `POST
//! /api/admin/reconcile/{entity}` runs one reconciliation pass on
//! demand, updating the same gauges the periodic worker updates, and is
//! gated as a privileged (`Action::Destructive`) action per
//! `agents/share/authorization-attributes.md` §4/§9 — the built-in
//! default policy grants it only to a machine peer (`svc=true`) or an
//! `access=admin` caller.
//!
//! Runs in its **own test binary**: it mints real PASETO v4.public
//! tokens against a throwaway key set, same shape as `tests/
//! concealment.rs`, so the process-wide verifier/policy/require-auth
//! `OnceLock`s don't leak into the other read suites.
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test force_reconcile -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use link_graph_service::app::App;
use link_graph_service::metrics::Metrics;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const SEED: [u8; 32] = [9; 32];
const ISSUER: &str = "authentication-service";
const AUDIENCE: &str = "main-x-service";

fn u(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

/// The published key set + its `kid`, for `LINK_GRAPH_PASETO_KEYS`.
fn keys_and_kid() -> (Value, String) {
    let public = SigningKey::from_bytes(&SEED).verifying_key().to_bytes();
    let kid = URL_SAFE_NO_PAD.encode(Sha256::digest(public));
    let keys = json!({
        "keys": [{ "kty": "OKP", "crv": "Ed25519", "use": "sig",
                   "kid": kid, "x": URL_SAFE_NO_PAD.encode(public) }]
    });
    (keys, kid)
}

/// Mint a bearer token with the given ABAC `attrs`.
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
        "email": "op@example.com", "name": "Operator",
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

fn bearer(token: &str) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    (
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    )
}

/// Spin up a tiny local HTTP server serving one fixed `subject_of`
/// (case → person) edge at `GET /edges`, matching the
/// `HttpAuthoritativeSource::fetch_all` contract (`{"edges": [LinkedEvent]}`).
/// Returns the URL to set `LINK_GRAPH_RECONCILE_URL_CASE` to.
async fn spawn_mock_source(edge_id: Uuid, case: Uuid, person: Uuid) -> String {
    let body = json!({
        "edges": [{
            "edge_id": edge_id.to_string(),
            "from_ref": format!("case:{case}"),
            "to_ref": format!("person:{person}"),
            "edge_kind": "subject_of",
            "role": null,
            "confidence": null,
            "provenance": "operator",
            "valid_from": null,
            "valid_to": null,
        }]
    });
    let app = axum::Router::new().route(
        "/edges",
        axum::routing::get(move || {
            let body = body.clone();
            async move { axum::Json(body) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock source");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("mock source serve");
    });
    format!("http://{addr}/edges")
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test force_reconcile -- --ignored`"]
async fn forced_pass_is_destructive_gated_and_updates_the_same_gauges() {
    let (keys, kid) = keys_and_kid();
    // Set all globals BEFORE the app boots (each is a OnceLock) — the
    // built-in default policy is enough here: read-allow for everyone,
    // destructive only for `svc=true` / `access=admin`.
    unsafe {
        std::env::set_var("LINK_GRAPH_REQUIRE_AUTH", "1");
        std::env::set_var("LINK_GRAPH_PASETO_KEYS", keys.to_string());
    }

    let edge_id = u(100);
    let case = u(1);
    let person = u(2);
    let url = spawn_mock_source(edge_id, case, person).await;
    unsafe {
        std::env::set_var("LINK_GRAPH_RECONCILE_URL_CASE", &url);
    }

    request::<App, _, _>(|request, _ctx| async move {
        // No token: 401, before any handler (and before any policy check)
        // runs — matching the blanket guard's existing posture.
        let anon = request.post("/api/admin/reconcile/case").await;
        assert_eq!(anon.status_code(), 401);

        // An authenticated caller with no admin/svc attribute: the
        // blanket guard's `read` check on `link_graph` would admit this
        // path, but the handler's own `Action::Destructive` check must
        // still refuse it (403) — same posture as case-service's bulk
        // `subject_of` dump (SEC-G1).
        let plain = mint(&kid, &[("dept", &["ops"])]);
        let (hk, hv) = bearer(&plain);
        let denied = request
            .post("/api/admin/reconcile/case")
            .add_header(hk, hv)
            .await;
        assert_eq!(denied.status_code(), 403, "{}", denied.text());

        // An unconfigured entity is a 404 (nothing to force), even for
        // an authorised admin caller.
        let admin = mint(&kid, &[("access", &["admin"])]);
        let (hk, hv) = bearer(&admin);
        let unconfigured = request
            .post("/api/admin/reconcile/worker")
            .add_header(hk, hv)
            .await;
        assert_eq!(unconfigured.status_code(), 404);

        // An admin caller against the configured `case` source: 200,
        // and the response reports the same divergence the periodic
        // worker would have found (one missing edge).
        let m = Metrics::global();
        let before = m
            .reconciliation_last_success_unixtime
            .with_label_values(&["case"])
            .get();
        let (hk, hv) = bearer(&admin);
        let ok = request
            .post("/api/admin/reconcile/case")
            .add_header(hk, hv)
            .await;
        assert_eq!(ok.status_code(), 200, "{}", ok.text());
        let body: Value = ok.json();
        assert_eq!(body["entity"], "case");
        assert_eq!(body["divergence_count"], 1, "one missing edge, {body}");

        // The same T-34/T-35 gauges the periodic worker updates moved:
        // last-success advanced, divergence recorded.
        assert!(
            m.reconciliation_last_success_unixtime
                .with_label_values(&["case"])
                .get()
                > before,
            "the forced pass must advance the last-success gauge"
        );
        assert_eq!(
            m.reconciliation_divergence
                .with_label_values(&["case"])
                .get(),
            1
        );

        // A second forced pass against the now-repaired read-model
        // converges (zero divergence) — proving this actually ran
        // `reconcile()`'s real repair, not a stub.
        let (hk, hv) = bearer(&admin);
        let converged = request
            .post("/api/admin/reconcile/case")
            .add_header(hk, hv)
            .await;
        assert_eq!(converged.status_code(), 200);
        let body: Value = converged.json();
        assert_eq!(body["divergence_count"], 0, "converged on the second pass");
    })
    .await;
}
