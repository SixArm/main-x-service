//! End-to-end proof of CP-T2: `list`, `search`, and `check-duplicates`
//! now run the same record-level ABAC decision `GET /{pid}` always has.
//!
//! `get_one`/`get_export` already call
//! [`crate::auth::authorize_record`](care_pathway_service::auth::authorize_record)
//! per pathway; before this change `list`/`search`/`check_duplicates` did
//! not, so a policy that **denies** read on some pathways (e.g. a
//! `resource.sensitive_setting`-gated rule) still let those pathways'
//! pid + name leak through the collection endpoints even though the
//! direct `GET /{pid}` would `403` — a collection read disclosing more
//! than the equivalent single read
//! (`agents/share/security.md` invariant 5).
//!
//! Its **own test binary**, same reason as `tests/masking.rs`:
//! `require_auth`/`policy`/`verifier` are process-wide `OnceLock`s.
//! `#[ignore]`d — boots the app, so it needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`. Run with
//! `cargo test --test list_search_check_duplicates_authz -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use care_pathway_service::app::App;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

const SEED: [u8; 32] = [7; 32];
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

fn sensitive_pathway() -> Value {
    json!({
        "name": "Community Mental Health Crisis Pathway",
        "care_setting": "MentalHealth",
        "provider_id": "trust-9",
        "provider_name": "Riverside Mental Health Trust",
        "pathway_code": "MH-CRISIS-01",
        "condition_codes": [{"system": "Icd10", "code": "F32"}]
    })
}

fn ordinary_pathway() -> Value {
    json!({
        "name": "Elective Hip Replacement Pathway",
        "care_setting": "Inpatient",
        "provider_id": "trust-2",
        "provider_name": "Northside Orthopaedic Centre",
        "pathway_code": "ORTHO-HIP-01"
    })
}

/// A `dept=outsider` caller reading `list`/`search`/`check-duplicates`
/// never sees the sensitive-setting pathway a `deny` rule blocks their
/// direct `GET` on — but does see the ordinary one, and an unrestricted
/// caller sees both everywhere. Proves the omission is scoped to the
/// denied caller/record pair, not a blanket collapse of the endpoints.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test list_search_check_duplicates_authz -- --ignored`"]
async fn denied_reader_never_sees_the_pathway_via_list_search_or_check_duplicates() {
    let (keys, kid) = keys_and_kid();
    // `dept=outsider` may not read a sensitive-setting pathway at all
    // (a `deny`, not a `mask` obligation — first-match-wins per
    // `agents/share/authorization-attributes.md` §4). Everyone else
    // reads under the default (read ⇒ allow); `access=write` may create.
    let policy = json!({
        "rules": [
            { "effect": "deny", "actions": ["read"],
              "when": { "dept": ["outsider"], "resource.sensitive_setting": ["true"] } },
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
            std::env::temp_dir().join(format!(
                "care-pathway-list-search-authz-index-{}",
                std::process::id()
            )),
        );
        std::env::set_var("CARE_PATHWAY_SEARCH_BOOT_REINDEX", "0");
    }

    request::<App, _, _>(|request, _ctx| async move {
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let sensitive = request
            .post("/api/care-pathways")
            .add_header(wk.clone(), wv.clone())
            .json(&sensitive_pathway())
            .await;
        assert_eq!(sensitive.status_code(), 200);
        let sensitive_pid = sensitive.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();
        let ordinary = request
            .post("/api/care-pathways")
            .add_header(wk, wv)
            .json(&ordinary_pathway())
            .await;
        assert_eq!(ordinary.status_code(), 200);
        let ordinary_pid = ordinary.json::<Value>()["pid"]
            .as_str()
            .unwrap()
            .to_string();

        // `access=write` lets this caller clear the *blanket* guard's
        // Write gate on `POST /check-duplicates` (`derive_action`
        // classifies it `Write`, not `Read` — see `src/auth.rs`); the
        // `dept=outsider` + `resource.sensitive_setting` deny rule is a
        // separate, *record-level* decision evaluated inside the
        // handler, and is what this test is actually proving.
        let (out_k, out_v) =
            auth_header(&mint(&kid, &[("dept", &["outsider"]), ("access", &["write"])]));

        // Control: the direct read is denied, confirming the policy
        // actually bites before trusting what the collection endpoints
        // omit.
        let direct = request
            .get(&format!("/api/care-pathways/{sensitive_pid}"))
            .add_header(out_k.clone(), out_v.clone())
            .await;
        assert_eq!(direct.status_code(), 403, "direct read is denied");

        // `list` omits the sensitive pathway for the denied caller, but
        // keeps the ordinary one.
        let listed: Vec<Value> = request
            .get("/api/care-pathways")
            .add_header(out_k.clone(), out_v.clone())
            .await
            .json();
        let listed_pids: Vec<&str> = listed
            .iter()
            .map(|r| r["pid"].as_str().unwrap())
            .collect();
        assert!(
            !listed_pids.contains(&sensitive_pid.as_str()),
            "list must not disclose a pathway the caller's direct read is denied on: {listed:?}"
        );
        assert!(
            listed_pids.contains(&ordinary_pid.as_str()),
            "list must still surface a pathway the policy does not deny: {listed:?}"
        );

        // `search` applies the identical filter.
        let searched: Vec<Value> = request
            .get("/api/care-pathways/search?q=pathway")
            .add_header(out_k.clone(), out_v.clone())
            .await
            .json();
        let searched_pids: Vec<&str> = searched
            .iter()
            .map(|r| r["pid"].as_str().unwrap())
            .collect();
        assert!(
            !searched_pids.contains(&sensitive_pid.as_str()),
            "search must not disclose a denied pathway: {searched:?}"
        );
        assert!(
            searched_pids.contains(&ordinary_pid.as_str()),
            "search must still surface an allowed pathway: {searched:?}"
        );

        // `check-duplicates` filters the same way: querying with the
        // sensitive pathway's own fields would otherwise return itself
        // as a near-perfect match.
        let dup_hits: Vec<Value> = request
            .post("/api/care-pathways/check-duplicates")
            .add_header(out_k.clone(), out_v.clone())
            .json(&sensitive_pathway())
            .await
            .json();
        assert!(
            dup_hits.is_empty(),
            "check-duplicates must not surface a denied pathway even as a duplicate hit: {dup_hits:?}"
        );

        // The unrestricted caller sees both everywhere — the omission is
        // scoped to the denied caller, not a blanket regression.
        let (ok_k, ok_v) = auth_header(&mint(&kid, &[]));
        let listed_ok: Vec<Value> = request
            .get("/api/care-pathways")
            .add_header(ok_k.clone(), ok_v.clone())
            .await
            .json();
        let listed_ok_pids: Vec<&str> = listed_ok
            .iter()
            .map(|r| r["pid"].as_str().unwrap())
            .collect();
        assert!(listed_ok_pids.contains(&sensitive_pid.as_str()));
        assert!(listed_ok_pids.contains(&ordinary_pid.as_str()));

        // `access=write` again, purely to clear the blanket guard's
        // Write gate on the POST — unrelated to the record-level
        // decision under test.
        let (ok_write_k, ok_write_v) = auth_header(&mint(&kid, &[("access", &["write"])]));
        let dup_hits_ok: Vec<Value> = request
            .post("/api/care-pathways/check-duplicates")
            .add_header(ok_write_k, ok_write_v)
            .json(&sensitive_pathway())
            .await
            .json();
        assert!(
            dup_hits_ok.iter().any(|h| h["pid"] == sensitive_pid),
            "an unrestricted caller must still find the real duplicate: {dup_hits_ok:?}"
        );
    })
    .await;

    unsafe {
        std::env::remove_var("CARE_PATHWAY_REQUIRE_AUTH");
        std::env::remove_var("CARE_PATHWAY_ABAC_POLICY");
        std::env::remove_var("CARE_PATHWAY_PASETO_KEYS");
    }
}
