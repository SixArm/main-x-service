//! End-to-end proof of SEC-PPM-1: `list`, `search`, and `check-duplicates`
//! now run the same record-level ABAC decision `GET /{pid}` always has.
//!
//! `get_one`/`get_export` already call
//! [`crate::auth::authorize_record`](project_portfolio_management_service::auth::authorize_record)
//! per plan (keyed on `resource.stage`, PPM-3); before this change
//! `list`/`search`/`check_duplicates` did not, so a policy that
//! **denies** read past some gate still let those plans' pid + name
//! leak through the collection endpoints even though the direct
//! `GET /{pid}` would `403` — a collection read disclosing more than
//! the equivalent single read (`agents/share/security.md` invariant 5).
//!
//! Its **own test binary**, same reason as `tests/masking.rs`:
//! `require_auth`/`policy`/`verifier` are process-wide `OnceLock`s.
//! `#[ignore]`d — boots the app, so it needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`. Run with
//! `cargo test --test list_search_check_duplicates_authz -- --ignored`.

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

fn gated_plan() -> Value {
    json!({
        "kind": "Project",
        "name": "Confidential Restructuring Program",
        "code": "PROJ-GATED-01"
    })
}

fn ordinary_plan() -> Value {
    json!({
        "kind": "Project",
        "name": "Ordinary Website Refresh Program",
        "code": "PROJ-ORDINARY-01"
    })
}

/// A `dept=outsider` caller reading `list`/`search`/`check-duplicates`
/// never sees the plan a `deny` rule blocks their direct `GET` on (once
/// it has passed gate 0) — but does see an ungated plan, and an
/// unrestricted caller sees both everywhere. Proves the omission is
/// scoped to the denied caller/record pair, not a blanket collapse of
/// the endpoints.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test list_search_check_duplicates_authz -- --ignored`"]
async fn denied_reader_never_sees_the_plan_via_list_search_or_check_duplicates() {
    let (keys, kid) = keys_and_kid();
    // `dept=outsider` may not read a plan that has passed gate 0 at all
    // (a `deny`, not a `mask` obligation — first-match-wins per
    // `agents/share/authorization-attributes.md` §4). Everyone else
    // reads under the default (read ⇒ allow); `access=write` may
    // create and advance gates.
    let policy = json!({
        "rules": [
            { "effect": "deny", "actions": ["read"],
              "when": { "dept": ["outsider"], "resource.stage": ["g0_concept"] } },
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
                "ppm-list-search-authz-index-{}",
                std::process::id()
            )),
        );
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_SEARCH_BOOT_REINDEX", "0");
    }

    request::<App, _, _>(|request, _ctx| async move {
        // `access=write` clears the blanket guard's Write gate on the
        // create/gate-review/check-duplicates POSTs.
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let gated = request
            .post("/api/plans")
            .add_header(wk.clone(), wv.clone())
            .json(&gated_plan())
            .await;
        assert_eq!(gated.status_code(), 200);
        let gated_pid = gated.json::<Value>()["pid"].as_str().unwrap().to_string();
        let ordinary = request
            .post("/api/plans")
            .add_header(wk.clone(), wv.clone())
            .json(&ordinary_plan())
            .await;
        assert_eq!(ordinary.status_code(), 200);
        let ordinary_pid = ordinary.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();

        // Advance the gated plan through gate 0 — this is what puts
        // `resource.stage = "g0_concept"` on it. The ordinary plan is
        // left untouched (`stage: None`), so it carries no `stage`
        // attribute at all and the deny rule never matches it.
        let review = request
            .post(&format!("/api/plans/{gated_pid}/gate-reviews"))
            .add_header(wk, wv)
            .json(&json!({ "gate": "g0_concept", "decision": "approved" }))
            .await;
        assert_eq!(review.status_code(), 200, "{}", review.text());

        // `access=write` again, purely to clear the blanket guard's
        // Write gate on `POST /check-duplicates` (`derive_action`
        // classifies it `Write`, not `Read`); the `dept=outsider` +
        // `resource.stage` deny rule is a separate, record-level
        // decision evaluated inside the handler, and is what this test
        // is actually proving.
        let (out_k, out_v) = auth_header(&mint(
            &kid,
            &[("dept", &["outsider"]), ("access", &["write"])],
        ));

        // Control: the direct read is denied, confirming the policy
        // actually bites before trusting what the collection endpoints
        // omit.
        let direct = request
            .get(&format!("/api/plans/{gated_pid}"))
            .add_header(out_k.clone(), out_v.clone())
            .await;
        assert_eq!(direct.status_code(), 403, "direct read is denied");

        // `list` omits the gated plan for the denied caller, but keeps
        // the ordinary one.
        let listed: Vec<Value> = request
            .get("/api/plans")
            .add_header(out_k.clone(), out_v.clone())
            .await
            .json();
        let listed_pids: Vec<&str> = listed.iter().map(|r| r["pid"].as_str().unwrap()).collect();
        assert!(
            !listed_pids.contains(&gated_pid.as_str()),
            "list must not disclose a plan the caller's direct read is denied on: {listed:?}"
        );
        assert!(
            listed_pids.contains(&ordinary_pid.as_str()),
            "list must still surface a plan the policy does not deny: {listed:?}"
        );

        // `search` applies the identical filter.
        let searched: Vec<Value> = request
            .get("/api/plans/search?q=Program")
            .add_header(out_k.clone(), out_v.clone())
            .await
            .json();
        let searched_pids: Vec<&str> = searched
            .iter()
            .map(|r| r["pid"].as_str().unwrap())
            .collect();
        assert!(
            !searched_pids.contains(&gated_pid.as_str()),
            "search must not disclose a denied plan: {searched:?}"
        );
        assert!(
            searched_pids.contains(&ordinary_pid.as_str()),
            "search must still surface an allowed plan: {searched:?}"
        );

        // `check-duplicates` filters the same way: querying with the
        // gated plan's own fields would otherwise return itself as a
        // near-perfect match.
        let dup_hits: Vec<Value> = request
            .post("/api/plans/check-duplicates")
            .add_header(out_k.clone(), out_v.clone())
            .json(&gated_plan())
            .await
            .json();
        assert!(
            dup_hits.is_empty(),
            "check-duplicates must not surface a denied plan even as a duplicate hit: {dup_hits:?}"
        );

        // The unrestricted caller sees both everywhere — the omission is
        // scoped to the denied caller, not a blanket regression.
        let (ok_k, ok_v) = auth_header(&mint(&kid, &[]));
        let listed_ok: Vec<Value> = request
            .get("/api/plans")
            .add_header(ok_k.clone(), ok_v.clone())
            .await
            .json();
        let listed_ok_pids: Vec<&str> = listed_ok
            .iter()
            .map(|r| r["pid"].as_str().unwrap())
            .collect();
        assert!(listed_ok_pids.contains(&gated_pid.as_str()));
        assert!(listed_ok_pids.contains(&ordinary_pid.as_str()));

        // `access=write` again, purely to clear the blanket guard's
        // Write gate on the POST — unrelated to the record-level
        // decision under test.
        let (ok_write_k, ok_write_v) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let dup_hits_ok: Vec<Value> = request
            .post("/api/plans/check-duplicates")
            .add_header(ok_write_k, ok_write_v)
            .json(&gated_plan())
            .await
            .json();
        assert!(
            dup_hits_ok.iter().any(|h| h["pid"] == gated_pid),
            "an unrestricted caller must still find the real duplicate: {dup_hits_ok:?}"
        );
    })
    .await;

    unsafe {
        std::env::remove_var("PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH");
        std::env::remove_var("PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY");
        std::env::remove_var("PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS");
    }
}
