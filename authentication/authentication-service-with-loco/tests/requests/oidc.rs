//! OIDC identity federation (EV-2) request tests. `#[ignore]`d: needs
//! PostgreSQL. Run with `cargo test --features oidc -- --ignored`.
//!
//! A minimal stub IdP is served from a local ephemeral-port HTTP
//! listener (mirroring `tests/enforcement.rs`'s PASETO key-set stub):
//! `/.well-known/openid-configuration`, `/jwks`, and `/token`. Its ID
//! tokens are signed ES256 (P-256 ECDSA) — a pure-Rust algorithm with
//! no `rsa`/RUSTSEC-2023-0071 exposure, dev-dependency only.

use authentication_service::app::App;
use axum::Json;
use axum::extract::State;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use loco_rs::testing::prelude::*;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use serde_json::{Value, json};
use serial_test::serial;
use std::sync::Arc;

/// A fixed, deterministic P-256 seed — test-only, never a real key.
const SEED: [u8; 32] = [7; 32];

fn signing_key() -> SigningKey {
    SigningKey::from_bytes((&SEED).into()).expect("valid seed")
}

fn b64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Sign `claims` as a compact ES256 JWT.
fn sign_id_token(claims: &Value, kid: &str) -> String {
    let header = json!({ "alg": "ES256", "typ": "JWT", "kid": kid });
    let signing_input = format!(
        "{}.{}",
        b64url(header.to_string().as_bytes()),
        b64url(claims.to_string().as_bytes())
    );
    let signature: Signature = signing_key().sign(signing_input.as_bytes());
    format!("{signing_input}.{}", b64url(&signature.to_bytes()))
}

/// The JWK for the stub IdP's public key.
fn jwk(kid: &str) -> Value {
    let point = signing_key().verifying_key().to_encoded_point(false);
    let (x, y) = (
        point.x().expect("uncompressed point has x"),
        point.y().expect("uncompressed point has y"),
    );
    json!({
        "kty": "EC", "crv": "P-256", "use": "sig", "alg": "ES256",
        "kid": kid, "x": b64url(x), "y": b64url(y),
    })
}

/// Shared state for the stub IdP: the next ID token it should hand
/// back from `/token` (set by the test, read once).
#[derive(Clone)]
struct StubState {
    id_token: Arc<std::sync::Mutex<Option<String>>>,
}

/// Serve a minimal stub OIDC provider. Returns the issuer base URL.
async fn serve_stub_idp() -> (String, StubState) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    let issuer = format!("http://{addr}");
    let state = StubState {
        id_token: Arc::new(std::sync::Mutex::new(None)),
    };

    let discovery_issuer = issuer.clone();
    let app_state = state.clone();
    let app = axum::Router::new()
        .route(
            "/.well-known/openid-configuration",
            axum::routing::get(move || {
                let issuer = discovery_issuer.clone();
                async move {
                    Json(json!({
                        "issuer": issuer,
                        "authorization_endpoint": format!("{issuer}/authorize"),
                        "token_endpoint": format!("{issuer}/token"),
                        "jwks_uri": format!("{issuer}/jwks"),
                        "response_types_supported": ["code"],
                        "subject_types_supported": ["public"],
                        "id_token_signing_alg_values_supported": ["ES256"],
                    }))
                }
            }),
        )
        .route(
            "/jwks",
            axum::routing::get(|| async { Json(json!({ "keys": [jwk("test-kid")] })) }),
        )
        .route(
            "/token",
            axum::routing::post(move |State(state): State<StubState>| async move {
                let id_token = state
                    .id_token
                    .lock()
                    .expect("lock")
                    .clone()
                    .expect("test set an ID token before exercising /token");
                Json(json!({
                    "access_token": "stub-access-token",
                    "token_type": "Bearer",
                    "id_token": id_token,
                    "expires_in": 300,
                }))
            }),
        )
        .with_state(app_state);

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve stub IdP");
    });
    (issuer, state)
}

/// Configure the OIDC env vars for one test. Cleared at the end by
/// the caller (`#[serial]` avoids cross-test races on these globals —
/// `OidcConfig::from_env` reads them fresh per request, so there is no
/// boot-time-only cache to contend with, unlike the PASETO verifier).
fn set_oidc_env(issuer: &str, redirect: &str, claim_map: Option<&str>, jit: bool) {
    // SAFETY: `tests/` binaries are not subject to the lib crate's
    // `#![forbid(unsafe_code)]`; `#[serial]` on every test in this
    // file prevents concurrent env mutation.
    unsafe {
        std::env::set_var("AUTH_OIDC_ISSUER_URL", issuer);
        std::env::set_var("AUTH_OIDC_CLIENT_ID", "test-client");
        std::env::set_var("AUTH_OIDC_CLIENT_SECRET", "test-secret");
        std::env::set_var("AUTH_OIDC_REDIRECT_URL", redirect);
        if let Some(map) = claim_map {
            std::env::set_var("AUTH_OIDC_CLAIM_MAP", map);
        } else {
            std::env::remove_var("AUTH_OIDC_CLAIM_MAP");
        }
        std::env::set_var("AUTH_OIDC_JIT_PROVISIONING", if jit { "1" } else { "0" });
    }
}

fn clear_oidc_env() {
    // SAFETY: see `set_oidc_env`.
    unsafe {
        std::env::remove_var("AUTH_OIDC_ISSUER_URL");
        std::env::remove_var("AUTH_OIDC_CLIENT_ID");
        std::env::remove_var("AUTH_OIDC_CLIENT_SECRET");
        std::env::remove_var("AUTH_OIDC_REDIRECT_URL");
        std::env::remove_var("AUTH_OIDC_CLAIM_MAP");
        std::env::remove_var("AUTH_OIDC_JIT_PROVISIONING");
    }
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --features oidc -- --ignored`"]
async fn both_routes_are_404_when_federation_is_unconfigured() {
    clear_oidc_env();
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/api/auth/oidc/login").await.status_code(), 404);
        assert_eq!(
            request
                .get("/api/auth/oidc/callback?code=x&state=y")
                .await
                .status_code(),
            404
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --features oidc -- --ignored`"]
async fn login_redirects_to_the_idp_with_pkce_state_and_nonce() {
    let (issuer, _state) = serve_stub_idp().await;
    set_oidc_env(
        &issuer,
        "http://localhost:3000/api/auth/oidc/callback",
        None,
        false,
    );
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/auth/oidc/login").await;
        assert_eq!(response.status_code(), 303);
        let location = response
            .header("location")
            .to_str()
            .expect("location header")
            .to_string();
        assert!(
            location.starts_with(&format!("{issuer}/authorize")),
            "{location}"
        );
        assert!(location.contains("code_challenge="));
        assert!(location.contains("state="));
        assert!(location.contains("nonce="));
        let set_cookie = response
            .header("set-cookie")
            .to_str()
            .expect("cookie")
            .to_string();
        assert!(set_cookie.starts_with("__Host-mxi_oidc_flow="));
        assert!(set_cookie.contains("HttpOnly"));
    })
    .await;
    clear_oidc_env();
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --features oidc -- --ignored`"]
async fn callback_refuses_a_state_mismatch_before_ever_calling_the_idp() {
    let (issuer, _state) = serve_stub_idp().await;
    set_oidc_env(
        &issuer,
        "http://localhost:3000/api/auth/oidc/callback",
        None,
        false,
    );
    request::<App, _, _>(|request, _ctx| async move {
        let login_response = request.get("/api/auth/oidc/login").await;
        let flow_cookie = login_response
            .header("set-cookie")
            .to_str()
            .expect("cookie")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string();
        let response = request
            .get("/api/auth/oidc/callback?code=whatever&state=not-the-real-state")
            .add_header(
                axum::http::header::COOKIE,
                flow_cookie.parse::<axum::http::HeaderValue>().unwrap(),
            )
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
    clear_oidc_env();
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --features oidc -- --ignored`"]
async fn callback_reports_an_idp_error_without_touching_the_token_endpoint() {
    let (issuer, _state) = serve_stub_idp().await;
    set_oidc_env(
        &issuer,
        "http://localhost:3000/api/auth/oidc/callback",
        None,
        false,
    );
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .get("/api/auth/oidc/callback?error=access_denied")
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
    clear_oidc_env();
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --features oidc -- --ignored`"]
// The full round trip: login -> a real ES256-signed ID token from the
// stub IdP -> callback verifies it (signature + nonce, via
// `openidconnect`'s own verifier) -> claims map to attrs -> the
// existing account is found (JIT off, the safer default) -> the same
// session establishment a magic-link redemption uses. This is the one
// test that proves the whole sequence is wired correctly, not just
// that each piece type-checks.
async fn callback_establishes_a_session_from_a_real_signed_id_token() {
    use authentication_service::models::users;
    use sea_orm::IntoActiveModel;

    let (issuer, stub) = serve_stub_idp().await;
    let redirect_url = "http://localhost:3000/api/auth/oidc/callback";
    set_oidc_env(&issuer, redirect_url, Some(r#"{"dept":"dept"}"#), false);

    request::<App, _, _>(|request, ctx| async move {
        let email = "federated.user@example.com";
        users::Model::create_passwordless(&ctx.db, email, "Federated User")
            .await
            .expect("seed an existing account (JIT is off)");

        let login_response = request.get("/api/auth/oidc/login").await;
        let location = login_response
            .header("location")
            .to_str()
            .expect("location")
            .to_string();
        let parsed = url::Url::parse(&location).expect("valid authorize URL");
        let query: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        let state = query.get("state").expect("state").clone();
        let nonce = query.get("nonce").expect("nonce").clone();
        let flow_cookie = login_response
            .header("set-cookie")
            .to_str()
            .expect("cookie")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string();

        let now = chrono::Utc::now().timestamp();
        let claims = json!({
            "iss": issuer,
            "sub": "idp-subject-123",
            "aud": "test-client",
            "exp": now + 300,
            "iat": now,
            "nonce": nonce,
            "email": email,
            "email_verified": true,
            "dept": "cardiology",
        });
        *stub.id_token.lock().expect("lock") = Some(sign_id_token(&claims, "test-kid"));

        let callback_response = request
            .get(&format!(
                "/api/auth/oidc/callback?code=stub-code&state={state}"
            ))
            .add_header(
                axum::http::header::COOKIE,
                flow_cookie.parse::<axum::http::HeaderValue>().unwrap(),
            )
            .await;
        assert_eq!(
            callback_response.status_code(),
            303,
            "{:?}",
            callback_response.text()
        );
        let session_cookie = callback_response
            .header("set-cookie")
            .to_str()
            .expect("a session cookie was set")
            .to_string();
        assert!(session_cookie.contains("__Host-mxi_session="));

        let updated = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("account still exists");
        assert_eq!(
            updated.attrs().get("dept"),
            Some(&vec!["cardiology".to_string()]),
            "the mapped claim landed in users.attributes"
        );
        let _ = updated.into_active_model();
    })
    .await;
    clear_oidc_env();
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --features oidc -- --ignored`"]
async fn callback_without_a_flow_cookie_is_refused() {
    let (issuer, _state) = serve_stub_idp().await;
    set_oidc_env(
        &issuer,
        "http://localhost:3000/api/auth/oidc/callback",
        None,
        false,
    );
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .get("/api/auth/oidc/callback?code=whatever&state=whatever")
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
    clear_oidc_env();
}
