//! Request tests for the admin ABAC attribute-assignment surface
//! (`/api/auth/admin/users/{pid}/attributes`, spec §13).
//!
//! The DB-backed flow tests boot the loco app and need the PostgreSQL
//! instance from `config/test.yaml`; they are `#[ignore]`d so a checkout
//! without Postgres keeps `cargo test` green. Run them with:
//!
//! ```text
//! cargo test -- --ignored
//! ```

use authentication_service::{
    app::App,
    models::{auth_events, users},
};
use loco_rs::testing::prelude::*;
use loco_rs::{TestServer, app::AppContext};
use serial_test::serial;

use super::prepare_data::auth_header;

/// Sign up `email`, grant it `attributes`, then run the magic-link flow
/// so the returned bearer carries those attributes in its `attrs` claim
/// (session establishment copies `users.attributes` at redemption). This
/// is how the bootstrap admin (`access=admin`) gets an admin token.
async fn sign_in_with_attributes(
    request: &TestServer,
    ctx: &AppContext,
    email: &str,
    attributes: serde_json::Value,
) -> String {
    // Create the account (first magic link is minted with empty attrs).
    let signup = request
        .post("/api/auth/signup")
        .json(&serde_json::json!({ "email": email, "name": "admin" }))
        .await;
    assert_eq!(signup.status_code(), 200);

    // Grant the attributes, then request a fresh link and redeem it — the
    // new session/token now carries them.
    let user = users::Model::find_by_email(&ctx.db, email).await.unwrap();
    users::ActiveModel::from(user)
        .set_attributes(&ctx.db, attributes)
        .await
        .unwrap();
    let link = request
        .post("/api/auth/magic-link")
        .json(&serde_json::json!({ "email": email }))
        .await;
    assert_eq!(link.status_code(), 200);

    // The stored token is a hash (SEC-A9), so issue one whose plaintext we
    // hold — see `prepare_data::issue_magic_link`.
    let user = users::Model::find_by_email(&ctx.db, email).await.unwrap();
    let token = super::prepare_data::issue_magic_link(ctx, user).await;
    let redeem = request.get(&format!("/api/auth/magic-link/{token}")).await;
    assert_eq!(redeem.status_code(), 200);
    let body: authentication_service::views::auth::LoginResponse =
        serde_json::from_str(&redeem.text()).unwrap();
    body.token
}

/// An admin can replace a user's attributes; the change persists, an
/// audit row is written, and GET reflects it.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn admin_can_replace_and_show_user_attributes() {
    request::<App, _, _>(|request, ctx| async move {
        let admin_token = sign_in_with_attributes(
            &request,
            &ctx,
            "admin@loco.com",
            serde_json::json!({ "access": ["admin"] }),
        )
        .await;

        // A fresh target user with no attributes.
        request
            .post("/api/auth/signup")
            .json(&serde_json::json!({ "email": "target@loco.com", "name": "target" }))
            .await;
        let target_pid = users::Model::find_by_email(&ctx.db, "target@loco.com")
            .await
            .unwrap()
            .pid;

        // SEC-A8: give the target a live session that snapshotted the OLD
        // (empty) attributes — it must be revoked by the attribute change.
        use authentication_service::models::sessions;
        sessions::Model::issue(
            &ctx.db,
            "a8-target-session",
            target_pid,
            Some("test-agent".to_string()),
            serde_json::json!({}),
        )
        .await
        .expect("issue a target session");

        // PUT replaces the whole map.
        let (k, v) = auth_header(&admin_token);
        let put = request
            .put(&format!("/api/auth/admin/users/{target_pid}/attributes"))
            .add_header(k, v)
            .json(&serde_json::json!({
                "attributes": { "access": ["write"], "dept": ["cardiology"] }
            }))
            .await;
        assert_eq!(put.status_code(), 200, "admin PUT should succeed");

        // SEC-A8: the attribute change revoked the target's sessions, so the
        // stale-attribute session can no longer mint tokens.
        let target_sessions = sessions::Model::find_all_by_user_pid(&ctx.db, target_pid)
            .await
            .expect("target sessions should query");
        assert!(
            target_sessions.iter().all(|s| !s.is_active()),
            "SEC-A8: an attribute change must revoke the target's sessions"
        );

        // Persisted.
        let updated = users::Model::find_by_email(&ctx.db, "target@loco.com")
            .await
            .unwrap();
        assert_eq!(updated.attrs()["access"], vec!["write".to_string()]);
        assert_eq!(updated.attrs()["dept"], vec!["cardiology".to_string()]);

        // Audited.
        let events = auth_events::Model::for_subject(&ctx.db, target_pid, "target@loco.com")
            .await
            .unwrap();
        assert!(
            events
                .iter()
                .any(|e| e.event == auth_events::ATTRIBUTES_ASSIGNED),
            "an attributes_assigned audit row should exist"
        );

        // GET reflects the new map.
        let (k, v) = auth_header(&admin_token);
        let get = request
            .get(&format!("/api/auth/admin/users/{target_pid}/attributes"))
            .add_header(k, v)
            .await;
        assert_eq!(get.status_code(), 200);
    })
    .await;
}

/// A valid but non-admin token is rejected with `403`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn non_admin_token_is_forbidden() {
    request::<App, _, _>(|request, ctx| async move {
        // A signed-in user with no attributes (write-less, not admin).
        let user_token = sign_in_with_attributes(
            &request,
            &ctx,
            "plain@loco.com",
            serde_json::json!({ "access": ["write"] }),
        )
        .await;
        let pid = users::Model::find_by_email(&ctx.db, "plain@loco.com")
            .await
            .unwrap()
            .pid;

        let (k, v) = auth_header(&user_token);
        let put = request
            .put(&format!("/api/auth/admin/users/{pid}/attributes"))
            .add_header(k, v)
            .json(&serde_json::json!({ "attributes": { "access": ["admin"] } }))
            .await;
        assert_eq!(
            put.status_code(),
            403,
            "a non-admin token must not assign attributes"
        );
    })
    .await;
}

/// No bearer token at all is rejected with `401` (the extractor).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn missing_token_is_unauthorized() {
    request::<App, _, _>(|request, _ctx| async move {
        let put = request
            .put("/api/auth/admin/users/00000000-0000-0000-0000-000000000000/attributes")
            .json(&serde_json::json!({ "attributes": {} }))
            .await;
        assert_eq!(put.status_code(), 401);
    })
    .await;
}
