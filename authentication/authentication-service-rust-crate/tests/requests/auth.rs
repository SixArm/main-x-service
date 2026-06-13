//! Request tests for the passwordless magic-link surface (spec §6):
//!
//! - `POST /api/auth/signup`            — create account, issue magic link
//! - `POST /api/auth/magic-link`        — issue magic link (sign in)
//! - `GET  /api/auth/magic-link/{token}`— redeem link → RS256 token
//! - `GET  /api/auth/me`                — current user (bearer)
//! - `POST /api/auth/signout`           — revoke session (bearer)
//! - `GET  /.well-known/jwks.json`      — public keys
//!
//! The HTTP tests boot the loco app and therefore need the PostgreSQL
//! instance from `config/test.yaml` (DATABASE_URL overridable). They
//! are `#[ignore]`d so a checkout without Postgres keeps `cargo test`
//! green; run them with:
//!
//! ```text
//! cargo test -- --ignored
//! ```
//!
//! The route-table and request/response-shape tests at the bottom are
//! DB-free and always run.

use authentication_service::{app::App, models::users};
use loco_rs::testing::prelude::*;
use serial_test::serial;

use super::prepare_data;

// ---------------------------------------------------------------------------
// DB-backed HTTP flow tests (require PostgreSQL; see module docs).
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn can_signup_and_issue_magic_link() {
    request::<App, _, _>(|request, ctx| async move {
        let email = "test@loco.com";
        let payload = serde_json::json!({
            "email": email,
            "name": "loco",
        });

        let response = request.post("/api/auth/signup").json(&payload).await;
        assert_eq!(response.status_code(), 200, "Signup should succeed");

        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("signup should create the user");
        assert_eq!(user.name, "loco");
        assert!(
            user.magic_link_token.is_some(),
            "signup should issue a magic link token"
        );
        assert!(
            user.magic_link_expiration.is_some(),
            "magic link should carry an expiration"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn signup_with_existing_email_still_returns_200() {
    // Anti-enumeration (spec §6.1): an existing email must not be
    // distinguishable from a fresh one — it gets a new link, same 200.
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let payload = serde_json::json!({
            "email": "user1@example.com",
            "name": "duplicate",
        });
        let response = request.post("/api/auth/signup").json(&payload).await;
        assert_eq!(response.status_code(), 200, "Duplicate signup must be 200");

        let user = users::Model::find_by_email(&ctx.db, "user1@example.com")
            .await
            .expect("seeded user should exist");
        assert!(
            user.magic_link_token.is_some(),
            "existing account should still receive a fresh magic link"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn can_request_magic_link_for_existing_account() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let payload = serde_json::json!({ "email": "user1@example.com" });
        let response = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(response.status_code(), 200, "Magic link request should succeed");

        let user = users::Model::find_by_email(&ctx.db, "user1@example.com")
            .await
            .expect("seeded user should exist");
        assert!(
            user.magic_link_token.is_some(),
            "magic link token should be generated"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn magic_link_for_unknown_email_still_returns_200() {
    // Anti-enumeration (spec §6.2): unknown emails get the same 200.
    request::<App, _, _>(|request, _ctx| async move {
        let payload = serde_json::json!({ "email": "nobody@example.com" });
        let response = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(
            response.status_code(),
            200,
            "Unknown email must be indistinguishable (200)"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn can_redeem_magic_link_for_access_token() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let payload = serde_json::json!({ "email": "user1@example.com" });
        let response = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(response.status_code(), 200);

        let user = users::Model::find_by_email(&ctx.db, "user1@example.com")
            .await
            .expect("seeded user should exist");
        let token = user
            .magic_link_token
            .expect("magic link token should be generated");

        let redeem = request.get(&format!("/api/auth/magic-link/{token}")).await;
        assert_eq!(redeem.status_code(), 200, "Redemption should succeed");

        let body: serde_json::Value = serde_json::from_str(&redeem.text()).unwrap();
        assert_eq!(body["email"], "user1@example.com");
        assert_eq!(body["pid"], user.pid.to_string());
        assert_eq!(body["is_verified"], true, "redemption verifies the email");
        let access_token = body["token"].as_str().expect("token should be a string");

        // The issued token is a valid RS256 JWT for this service.
        let claims = authentication_service::auth::verify_token(access_token)
            .expect("issued token should verify against the local key");
        assert_eq!(claims.sub, user.pid.to_string());
        assert_eq!(claims.email, "user1@example.com");

        // Magic links are single-use (spec §6): the second redemption fails.
        let again = request.get(&format!("/api/auth/magic-link/{token}")).await;
        assert_eq!(again.status_code(), 401, "Magic links must be single-use");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn invalid_magic_link_token_is_rejected() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let response = request.get("/api/auth/magic-link/invalid-token").await;
        assert_eq!(response.status_code(), 401, "Invalid token must be rejected");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn can_get_current_user() {
    request::<App, _, _>(|request, ctx| async move {
        let logged_in = prepare_data::init_user_login(&request, &ctx).await;

        let (auth_key, auth_value) = prepare_data::auth_header(&logged_in.token);
        let response = request
            .get("/api/auth/me")
            .add_header(auth_key, auth_value)
            .await;
        assert_eq!(response.status_code(), 200, "/me should succeed with a bearer token");

        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(body["pid"], logged_in.user.pid.to_string());
        assert_eq!(body["email"], logged_in.user.email);
        assert_eq!(body["name"], logged_in.user.name);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn me_without_bearer_token_is_unauthorized() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/auth/me").await;
        assert_eq!(response.status_code(), 401, "/me requires a bearer token");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn signout_revokes_the_session() {
    request::<App, _, _>(|request, ctx| async move {
        let logged_in = prepare_data::init_user_login(&request, &ctx).await;

        let (auth_key, auth_value) = prepare_data::auth_header(&logged_in.token);
        let signout = request
            .post("/api/auth/signout")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(signout.status_code(), 200, "Signout should succeed");

        // The JWT signature is still valid, but the session is revoked
        // locally, so /me must now reject it (spec §6.4).
        let me = request
            .get("/api/auth/me")
            .add_header(auth_key, auth_value)
            .await;
        assert_eq!(me.status_code(), 401, "Revoked session must be rejected by /me");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn jwks_endpoint_publishes_the_signing_key() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/.well-known/jwks.json").await;
        assert_eq!(response.status_code(), 200, "JWKS must be public");

        let jwks: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let key = &jwks["keys"][0];
        assert_eq!(key["kty"], "RSA");
        assert_eq!(key["use"], "sig");
        assert_eq!(key["alg"], "RS256");
        // The published kid is the one stamped into token headers.
        assert_eq!(
            key["kid"].as_str().unwrap(),
            authentication_service::auth::keys().kid,
        );
    })
    .await;
}

// ---------------------------------------------------------------------------
// DB-free contract assertions (always run).
// ---------------------------------------------------------------------------

#[test]
fn route_table_covers_the_magic_link_surface() {
    let routes = authentication_service::controllers::auth::routes();
    assert_eq!(routes.prefix.as_deref(), Some("/api/auth"));
    let uris: Vec<&str> = routes.handlers.iter().map(|h| h.uri.as_str()).collect();
    for expected in ["/signup", "/magic-link", "/magic-link/{token}", "/me", "/signout"] {
        assert!(uris.contains(&expected), "missing route {expected}; have {uris:?}");
    }

    let jwks = authentication_service::controllers::jwks::routes();
    assert_eq!(jwks.prefix.as_deref(), Some("/.well-known"));
    assert!(jwks.handlers.iter().any(|h| h.uri == "/jwks.json"));
}

#[test]
fn signup_params_accept_an_optional_name() {
    use authentication_service::controllers::auth::SignupParams;

    let with_name: SignupParams =
        serde_json::from_value(serde_json::json!({"email": "a@example.com", "name": "A"}))
            .expect("email + name should deserialize");
    assert_eq!(with_name.name.as_deref(), Some("A"));

    let without_name: SignupParams =
        serde_json::from_value(serde_json::json!({"email": "a@example.com"}))
            .expect("name should be optional");
    assert!(without_name.name.is_none());

    serde_json::from_value::<SignupParams>(serde_json::json!({"name": "A"}))
        .expect_err("email should be required");
}
