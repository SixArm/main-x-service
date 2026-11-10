//! Record-level read authorization on every collection read (ORG-T2,
//! `agents/share/security.md` invariant 5): list, search,
//! `check-duplicates`, the create-time `409`, the batch scan's response
//! and the stored review queue omit an organization the caller may not
//! read, and keep only the evidence aggregate for one the caller may read
//! only masked.
//!
//! Its **own test binary**, because `require_auth` / `policy` / `verifier`
//! are process-wide `OnceLock`s (see `tests/masking.rs`).
//!
//! `#[ignore]`d — boots the app, so it needs PostgreSQL via `DB_URL`. Run
//! with `cargo test --test visibility -- --ignored`.
#![cfg(feature = "paseto")]

use axa_test::prelude::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use organization_service::app::App;
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

/// Seed an organization through the model layer, past the create-time
/// duplicate check, and return its pid.
async fn seed(ctx: &axa::AppContext, name: &str, jurisdiction: &str) -> String {
    let org: organization_matcher::Organization = serde_json::from_value(json!({
        "name": name,
        "url": "https://holdings.example",
        "jurisdiction": jurisdiction,
    }))
    .unwrap();
    organization_service::streaming::create_and_emit(&ctx.db, &org, None)
        .await
        .expect("seed")
        .pid
        .to_string()
}

fn pids_in(body: &Value, key: &str) -> Vec<String> {
    body.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v[key].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with `cargo test --test visibility -- --ignored`"]
async fn collection_reads_omit_hidden_and_reduce_masked_records() {
    let (keys, kid) = keys_and_kid();
    // Read is denied on Jersey records and masked on Isle of Man ones;
    // everything else reads in full. `access=admin` may write and run the
    // destructive batch scan.
    let policy = json!({
        "rules": [
            { "effect": "deny", "actions": ["read"],
              "when": { "resource.jurisdiction": ["je"] } },
            { "effect": "allow", "actions": ["read"],
              "when": { "resource.jurisdiction": ["im"] }, "obligations": ["mask"] },
            { "effect": "allow", "actions": ["write", "destructive"],
              "when": { "access": ["admin"] } }
        ]
    });
    unsafe {
        std::env::set_var("ORGANIZATION_REQUIRE_AUTH", "1");
        std::env::set_var("ORGANIZATION_PASETO_KEYS", keys.to_string());
        std::env::set_var("ORGANIZATION_ABAC_POLICY", policy.to_string());
        std::env::set_var(
            "ORGANIZATION_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!(
                "organization-visibility-index-{}",
                std::process::id()
            )),
        );
        std::env::set_var("ORGANIZATION_SEARCH_BOOT_REINDEX", "0");
    }

    request::<App, _, _>(|request, ctx| async move {
        let token = format!("vis{}", uuid::Uuid::new_v4().simple());
        let name = format!("{token} Holdings");
        let full = seed(&ctx, &name, "GB").await;
        let masked = seed(&ctx, &name, "IM").await;
        let hidden = seed(&ctx, &name, "JE").await;
        let (rk, rv) = auth_header(&mint(&kid, &[]));
        let (ak, av) = auth_header(&mint(&kid, &[("access", &["admin"])]));

        // The single read: the hidden record is refused outright.
        let one = request
            .get(&format!("/api/organizations/{hidden}"))
            .add_header(rk.clone(), rv.clone())
            .await;
        assert_eq!(
            one.status_code(),
            403,
            "the policy denies the hidden record"
        );

        // List: newest first, so this run's rows are on the first page.
        let list: Value = request
            .get("/api/organizations?limit=50")
            .add_header(rk.clone(), rv.clone())
            .await
            .json();
        let listed = pids_in(&list, "pid");
        assert!(listed.contains(&full) && listed.contains(&masked), "{list}");
        assert!(
            !listed.contains(&hidden),
            "list must omit the hidden record"
        );

        // Search.
        let found: Value = request
            .get(&format!("/api/organizations/search?q={token}"))
            .add_header(rk.clone(), rv.clone())
            .await
            .json();
        let found_pids = pids_in(&found, "pid");
        assert!(
            found_pids.contains(&full) && found_pids.contains(&masked),
            "{found}"
        );
        assert!(
            !found_pids.contains(&hidden),
            "search must omit the hidden record"
        );

        // check-duplicates: the hidden candidate is not scored or shown,
        // and the masked one keeps only its evidence's aggregate.
        let probe = json!({ "name": name, "url": "https://holdings.example" });
        // A POST, so the blanket guard needs `write`; the record-level
        // read decision is the same for every caller here.
        let dups: Value = request
            .post("/api/organizations/check-duplicates")
            .add_header(ak.clone(), av.clone())
            .json(&probe)
            .await
            .json();
        let dup_pids = pids_in(&dups, "pid");
        assert!(
            dup_pids.contains(&full) && dup_pids.contains(&masked),
            "{dups}"
        );
        assert!(
            !dup_pids.contains(&hidden),
            "check-duplicates must omit the hidden record"
        );
        let of = |pid: &str| {
            dups.as_array()
                .unwrap()
                .iter()
                .find(|c| c["pid"] == pid)
                .cloned()
                .unwrap()
        };
        assert!(
            !of(&full)["evidence"]["components"]
                .as_array()
                .unwrap()
                .is_empty(),
            "a readable candidate keeps its per-term evidence"
        );
        assert!(
            of(&masked)["evidence"]["components"]
                .as_array()
                .unwrap()
                .is_empty(),
            "a masked candidate keeps only the aggregate: {dups}"
        );

        // The create-time 409 names only what the caller may read.
        let conflict = request
            .post("/api/organizations")
            .add_header(ak.clone(), av.clone())
            .json(&probe)
            .await;
        assert_eq!(conflict.status_code(), 409, "{}", conflict.text());
        let errors: Value = conflict.json();
        let conflict_pids = pids_in(&errors["errors"], "pid");
        assert!(
            !conflict_pids.contains(&hidden),
            "the 409 must omit the hidden record: {errors}"
        );

        // A probe whose only near match is hidden is not a 409, or the
        // refusal would disclose the record.
        let solo = format!("solo{} Holdings", uuid::Uuid::new_v4().simple());
        let _hidden_solo = seed(&ctx, &solo, "JE").await;
        let created = request
            .post("/api/organizations")
            .add_header(ak.clone(), av.clone())
            .json(&json!({ "name": solo, "url": "https://holdings.example" }))
            .await;
        assert_eq!(created.status_code(), 200, "{}", created.text());

        // The batch scan stores every pair but shows only readable ones.
        let scan: Value = request
            .post("/api/organizations/deduplicate")
            .add_header(ak.clone(), av.clone())
            // The fixtures differ in jurisdiction, which drives their
            // visibility, so the matcher's own verdict may reject them; an
            // explicit threshold reports the pairs.
            .json(&json!({ "threshold": 0.5 }))
            .await
            .json();
        let names_hidden = |items: &Value| {
            items.as_array().unwrap().iter().any(|i| {
                i["organization_id_a"] == hidden.as_str()
                    || i["organization_id_b"] == hidden.as_str()
            })
        };
        assert!(
            !names_hidden(&scan["review_items"]),
            "the scan response must omit hidden pairs"
        );
        let pair_in = |items: &Value| {
            items
                .as_array()
                .unwrap()
                .iter()
                .find(|i| {
                    let ids = [
                        i["organization_id_a"].as_str(),
                        i["organization_id_b"].as_str(),
                    ];
                    ids.contains(&Some(full.as_str())) && ids.contains(&Some(masked.as_str()))
                })
                .cloned()
        };
        let scanned = pair_in(&scan["review_items"]).expect("the readable pair is reported");
        assert!(
            scanned["evidence"]["components"]
                .as_array()
                .is_none_or(Vec::is_empty),
            "a pair naming a masked record keeps only the aggregate: {scanned}"
        );

        // The stored review queue, newest first.
        let queue: Value = request
            .get("/api/organizations/review-queue?limit=500")
            .add_header(rk.clone(), rv.clone())
            .await
            .json();
        assert!(
            !names_hidden(&queue["items"]),
            "the review queue must omit hidden pairs"
        );
        let stored = pair_in(&queue["items"]).expect("the readable pair is listed");
        assert!(
            stored["evidence"]["components"]
                .as_array()
                .is_none_or(Vec::is_empty),
            "{stored}"
        );

        // ORG-T6: the FHIR surface takes the REST read's decision. A
        // denied record is the REST read's `403`, as an OperationOutcome.
        let denied = request
            .get(&format!("/fhir/Organization/{hidden}"))
            .add_header(rk.clone(), rv.clone())
            .await;
        assert_eq!(denied.status_code(), 403, "a denied FHIR read is refused");
        let outcome: Value = denied.json();
        assert_eq!(outcome["resourceType"], "OperationOutcome", "{outcome}");
        assert_eq!(outcome["issue"][0]["code"], "forbidden", "{outcome}");

        // A `mask` obligation serves the masked record over FHIR too.
        let phone = "+44 1624 555 0199";
        let with_phone: organization_matcher::Organization = serde_json::from_value(json!({
            "name": format!("{token} Telephony"),
            "jurisdiction": "IM",
            "telephone": phone,
        }))
        .unwrap();
        let phoned = organization_service::streaming::create_and_emit(&ctx.db, &with_phone, None)
            .await
            .expect("seed")
            .pid
            .to_string();
        let read: Value = request
            .get(&format!("/fhir/Organization/{phoned}"))
            .add_header(rk.clone(), rv.clone())
            .await
            .json();
        let shown = read.to_string();
        assert!(!shown.contains(phone), "the FHIR read must mask: {read}");
        assert!(
            shown.contains("0199"),
            "masking keeps the last four: {read}"
        );

        // FHIR search leaves the denied record out of the Bundle, and its
        // `total` counts only what is shown.
        let bundle: Value = request
            .get(&format!("/fhir/Organization?name={token}%20Holdings"))
            .add_header(rk.clone(), rv.clone())
            .await
            .json();
        let ids: Vec<String> = bundle["entry"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|e| e["resource"]["id"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        assert!(ids.contains(&full), "{bundle}");
        assert!(ids.contains(&masked), "{bundle}");
        assert!(
            !ids.contains(&hidden),
            "FHIR search must omit the hidden record: {bundle}"
        );
        assert_eq!(bundle["total"].as_u64(), Some(ids.len() as u64), "{bundle}");
    })
    .await;
}
