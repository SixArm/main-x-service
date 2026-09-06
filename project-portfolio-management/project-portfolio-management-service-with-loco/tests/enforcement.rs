//! End-to-end **blanket enforcement + ABAC** proof over the real router
//! with `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` **on** — the "activation proof"
//! (AU-2).
//!
//! The DB-free unit tests in `src/auth.rs` pin the pure `enforce` /
//! policy-evaluation matrix. What they cannot show is that the
//! middleware is actually *wired* onto the router loco builds, and that
//! a real request with (or without) a valid PASETO gets the right
//! status. Activation is the moment this service stops being open, so it
//! deserves a test that fails if the wiring is ever lost.
//!
//! Its **own test binary**, because `require_auth` / `policy` /
//! `verifier` are process-wide `OnceLock`s: the enforcement-*off*
//! request suite would otherwise cache the flag before this test sets
//! it.
//!
//! `#[ignore]`d — boots the app, so it needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`. Run with
//! `cargo test --test enforcement -- --ignored`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::SigningKey;
use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use rusty_paseto::core::{Footer, Key, Paseto, PasetoAsymmetricPrivateKey, Payload, Public, V4};
use serde_json::{Value, json};
use serial_test::serial;
use sha2::{Digest, Sha256};

/// Fixed signing seed, so the published key set and the minted tokens
/// agree without a running auth service.
const SEED: [u8; 32] = [31; 32];
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

/// A minimal valid plan payload with a unique name, so a create
/// that reaches the handler is not turned away as a duplicate.
fn plan_body() -> Value {
    let unique = uuid::Uuid::new_v4().simple().to_string();
    json!({ "name": format!("{unique} Enforcement Plan") })
}

/// With enforcement on: public paths stay open, protected paths need a
/// token, and the default policy lets any authenticated caller read
/// while requiring `access=write` to write.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test enforcement -- --ignored`"]
async fn enforcement_on_gates_the_real_router() {
    let (keys, kid) = keys_and_kid();
    // Set BEFORE the app boots — the flag, key set and policy are read
    // into process-wide `OnceLock`s on first use. No
    // `PROJECT_PORTFOLIO_MANAGEMENT_ABAC_POLICY` ⇒ the built-in default policy (read
    // allow, mutation deny, `access=write` writes).
    unsafe {
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH", "1");
        std::env::set_var("PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS", keys.to_string());
    }

    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(
            request.get("/_health").await.status_code(),
            200,
            "a public path stays open with enforcement on"
        );
        assert_eq!(
            request.get("/metrics.prom").await.status_code(),
            200,
            "metrics stay scrapeable without a token"
        );

        assert_eq!(
            request.get("/api/plans").await.status_code(),
            401,
            "a protected read without a token is 401"
        );
        assert_eq!(
            request
                .post("/api/plans")
                .json(&plan_body())
                .await
                .status_code(),
            401,
            "a protected write without a token is 401"
        );

        // The time-based-analysis surface says who moved what, when, so
        // it is behind the same guard — pinned explicitly rather than
        // assumed to be covered by the `/api/plans` check above.
        {
            let nil = "00000000-0000-0000-0000-000000000000";
            for path in [
                "/api/flow-classes".to_string(),
                format!("/api/plans/{nil}/time-analysis"),
                format!("/api/plans/{nil}/constraints"),
                format!("/api/plans/{nil}/aging-wip"),
                format!("/api/plans/{nil}/flow"),
                format!("/api/plans/{nil}/cumulative-flow"),
                format!("/api/plans/{nil}/forecast"),
                format!("/api/plans/{nil}/rollup"),
                format!("/api/plans/{nil}/tasks/{nil}/transitions"),
                format!("/api/plans/{nil}/tasks/{nil}/time-analysis"),
            ] {
                assert_eq!(
                    request.get(&path).await.status_code(),
                    401,
                    "un-authed {path} must be 401 when enforcement is on"
                );
            }
        }

        // A garbage bearer is a 401, not a 500 — the guard must reject
        // malformed credentials rather than fall over on them.
        let (bk, bv) = auth_header("not-a-token");
        assert_eq!(
            request
                .get("/api/plans")
                .add_header(bk, bv)
                .await
                .status_code(),
            401,
            "a malformed bearer is 401"
        );

        // Valid token, no attributes: the default policy allows the read
        // and denies the write — 403, distinct from the 401 above. That
        // split is the contract in authorization-attributes.md §5.
        let (rk, rv) = auth_header(&mint(&kid, &[]));
        assert_eq!(
            request
                .get("/api/plans")
                .add_header(rk.clone(), rv.clone())
                .await
                .status_code(),
            200,
            "any authenticated caller may read"
        );
        assert_eq!(
            request
                .post("/api/plans")
                .add_header(rk, rv)
                .json(&plan_body())
                .await
                .status_code(),
            403,
            "a caller without access=write is denied by policy, not by authentication"
        );

        // `access=write` reaches the handler and does real database work.
        let (wk, wv) = auth_header(&mint(&kid, &[("access", &["write"])]));
        assert_eq!(
            request
                .post("/api/plans")
                .add_header(wk, wv)
                .json(&plan_body())
                .await
                .status_code(),
            200,
            "access=write may create"
        );

        // SEC-PPM-3: the oversight bulk-read endpoints (board pack,
        // board investments, auditor trail, evidence pack) are gated as
        // a privileged (Action::Destructive) read, not just the
        // blanket read-allow default a plain caller gets on `/api/plans`
        // above — the built-in default policy grants `destructive` only
        // to `access=admin` or `svc=true`.
        let oversight_paths = [
            "/api/board/pack",
            "/api/board/investments",
            "/api/auditor/trail",
            "/api/auditor/evidence-pack",
        ];
        for path in oversight_paths {
            let (pk, pv) = auth_header(&mint(&kid, &[]));
            assert_eq!(
                request.get(path).add_header(pk, pv).await.status_code(),
                403,
                "{path} denies a plain-read-tier caller (SEC-PPM-3)"
            );
        }
        for path in oversight_paths {
            let (ak, av) = auth_header(&mint(&kid, &[("access", &["admin"])]));
            assert_eq!(
                request.get(path).add_header(ak, av).await.status_code(),
                200,
                "{path} allows an access=admin caller (SEC-PPM-3)"
            );
        }
        for path in oversight_paths {
            let (sk, sv) = auth_header(&mint(&kid, &[("svc", &["true"])]));
            assert_eq!(
                request.get(path).add_header(sk, sv).await.status_code(),
                200,
                "{path} allows a svc=true caller (SEC-PPM-3)"
            );
        }
    })
    .await;

    unsafe {
        std::env::remove_var("PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH");
        std::env::remove_var("PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS");
    }
}
