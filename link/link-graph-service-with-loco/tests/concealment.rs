//! DB-gated end-to-end proof of the load-bearing §10 governance invariant
//! (spec T-16): with enforcement on, an authenticated caller the ABAC
//! policy grants case-read sees a `subject_of` (case↔person) edge — and
//! that read is audited — while an authenticated caller the policy denies
//! case-read has the same edge **concealed** (an ordinary affiliation
//! stays visible to both). Complements the unit tests in `src/auth.rs`
//! and the blanket-guard test in `tests/governance.rs`.
//!
//! Runs in its **own test binary**: it mints real PASETO v4.public tokens
//! against a throwaway key set and installs a restrictive ABAC policy, all
//! read once into process globals — so this must not share the verifier /
//! policy / require-auth `OnceLock`s with the other read suites.
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test concealment -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use link_graph_service::app::App;
use link_graph_service::events::{Envelope, apply_event};
use link_graph_service::models::_entities::audit_log;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const SEED: [u8; 32] = [7; 32];
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

fn linked_env(edge_id: Uuid, from: &str, to: &str, edge_kind: &str, seq: i64) -> Envelope {
    serde_json::from_value(json!({
        "entity": from.split(':').next().unwrap(),
        "pid": from.split(':').nth(1).unwrap(),
        "kind": "linked",
        "seq": seq,
        "occurred_at": "2026-07-10T10:00:00Z",
        "data": {
            "edge_id": edge_id.to_string(), "from_ref": from, "to_ref": to,
            "edge_kind": edge_kind, "provenance": "operator"
        }
    }))
    .unwrap()
}

fn bearer(token: &str) -> (axum::http::HeaderName, axum::http::HeaderValue) {
    (
        axum::http::header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    )
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test concealment -- --ignored`"]
async fn case_read_sees_governed_edges_that_others_have_concealed() {
    let (keys, kid) = keys_and_kid();
    // A restrictive policy: any authenticated caller may read the
    // aggregator (so both pass the blanket guard), but case-read — which
    // gates the governed subject_of edge — needs `dept=cases`.
    let policy = json!({
        "rules": [
            { "effect": "allow", "actions": ["read"], "when": { "entity": ["link_graph"] } },
            { "effect": "allow", "actions": ["read"], "when": { "entity": ["case"], "dept": ["cases"] } },
            { "effect": "deny",  "actions": ["read"], "when": { "entity": ["case"] } }
        ]
    });
    // Set all three globals BEFORE the app boots (each is a OnceLock).
    unsafe {
        std::env::set_var("LINK_GRAPH_REQUIRE_AUTH", "1");
        std::env::set_var("LINK_GRAPH_PASETO_KEYS", keys.to_string());
        std::env::set_var("LINK_GRAPH_ABAC_POLICY", policy.to_string());
    }

    request::<App, _, _>(|request, ctx| async move {
        let case = format!("case:{}", u(1));
        let person = format!("person:{}", u(2));
        let worker = format!("worker:{}", u(3));
        let org = format!("organization:{}", u(4));
        apply_event(&ctx.db, linked_env(u(10), &case, &person, "subject_of", 1))
            .await
            .unwrap();
        apply_event(&ctx.db, linked_env(u(11), &worker, &org, "employed_by", 2))
            .await
            .unwrap();

        let case_reader = mint(&kid, &[("dept", &["cases"])]);
        let other = mint(&kid, &[("dept", &["hr"])]);

        // The case-authorised caller sees BOTH edges.
        let (hk, hv) = bearer(&case_reader);
        let body: Value = request.get("/api/edges").add_header(hk, hv).await.json();
        let edges = body["data"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 2, "case-reader sees affiliation + subject_of");
        assert!(edges.iter().any(|e| e["kind"] == "subject_of"));

        // ...and that governed read was audited with the caller's sub.
        let reads = audit_log::Entity::find()
            .filter(audit_log::Column::Action.eq("read_edge"))
            .all(&ctx.db)
            .await
            .unwrap();
        assert_eq!(reads.len(), 1, "the surfaced subject_of read is audited");
        assert_eq!(reads[0].edge_kind.as_deref(), Some("subject_of"));
        assert_eq!(
            reads[0].actor.as_deref(),
            Some("11111111-1111-1111-1111-111111111111")
        );
        // AU-3: the caller's source address is on the row. It used to be
        // `None` unconditionally — a governance audit that records who but
        // never from where answers half the question it exists to answer.
        assert!(
            reads[0].user_ip.as_deref().is_some_and(|ip| !ip.is_empty()),
            "the governed read must record the caller's address: {:?}",
            reads[0].user_ip
        );

        // The non-case caller passes the blanket guard but the subject_of
        // edge is concealed — only the affiliation shows.
        let (hk, hv) = bearer(&other);
        let body: Value = request.get("/api/edges").add_header(hk, hv).await.json();
        let edges = body["data"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1, "non-case caller sees only the affiliation");
        assert_eq!(edges[0]["kind"], "employed_by");
        assert!(!edges.iter().any(|e| e["kind"] == "subject_of"));

        // The concealed read added no further read_edge audit row.
        let reads = audit_log::Entity::find()
            .filter(audit_log::Column::Action.eq("read_edge"))
            .all(&ctx.db)
            .await
            .unwrap();
        assert_eq!(reads.len(), 1, "a concealed read audits nothing");
    })
    .await;
}
