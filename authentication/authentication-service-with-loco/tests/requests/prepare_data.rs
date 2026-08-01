//! Shared helpers for request tests: drive the passwordless magic-link
//! flow end-to-end to obtain a signed-in user + PASETO bearer token.

use authentication_service::{models::users, views::auth::LoginResponse};
use axum::http::{HeaderName, HeaderValue};
use loco_rs::{TestServer, app::AppContext};
use sea_orm::IntoActiveModel;

const USER_EMAIL: &str = "test@loco.com";
const USER_NAME: &str = "loco";

/// A signed-in test subject: the persisted `users` row plus the PASETO
/// bearer token minted by redeeming their magic link.
pub struct LoggedInUser {
    /// The persisted user record (refetched after redemption).
    pub user: users::Model,
    /// The `v4.public` access token to send as `Authorization: Bearer …`.
    pub token: String,
}

/// Issue a magic link for `user` and return its **plaintext** token.
///
/// The token cannot be read back out of the database: SEC-A9 stores only
/// `hash(token)`, and redemption hashes what the caller presents. The
/// plaintext exists in exactly one place — the model
/// [`create_magic_link`](users::ActiveModel::create_magic_link) returns,
/// which is where production takes it from to build the email. Tests
/// therefore take it from the same place; reading `users.magic_link_token`
/// yields the hash, and redeeming *that* is a `401`.
pub async fn issue_magic_link(ctx: &AppContext, user: users::Model) -> String {
    user.into_active_model()
        .create_magic_link(&ctx.db)
        .await
        .expect("issuing a magic link should succeed")
        .magic_link_token
        .expect("create_magic_link returns the plaintext token")
}

/// Sign up a fresh account, obtain a usable magic link, and redeem it
/// over HTTP for an access token.
pub async fn init_user_login(request: &TestServer, ctx: &AppContext) -> LoggedInUser {
    let signup_payload = serde_json::json!({
        "email": USER_EMAIL,
        "name": USER_NAME,
    });
    let signup_response = request.post("/api/auth/signup").json(&signup_payload).await;
    assert_eq!(
        signup_response.status_code(),
        200,
        "Signup request should succeed"
    );

    let user = users::Model::find_by_email(&ctx.db, USER_EMAIL)
        .await
        .expect("signup should have created the user");
    assert!(
        user.magic_link_token.is_some(),
        "signup should have issued a magic link"
    );
    let magic_link_token = issue_magic_link(ctx, user).await;

    let verify_response = request
        .get(&format!("/api/auth/magic-link/{magic_link_token}"))
        .await;
    assert_eq!(
        verify_response.status_code(),
        200,
        "Magic link redemption should succeed"
    );

    let login_response: LoginResponse = serde_json::from_str(&verify_response.text())
        .expect("redemption should return a LoginResponse");

    LoggedInUser {
        user: users::Model::find_by_email(&ctx.db, USER_EMAIL)
            .await
            .expect("user should still exist after login"),
        token: login_response.token,
    }
}

/// Build the `Authorization: Bearer <token>` header pair to attach to a
/// request for the bearer-gated endpoints (`/me`, `/signout`, account…).
pub fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let auth_header_value = HeaderValue::from_str(&format!("Bearer {}", &token)).unwrap();

    (HeaderName::from_static("authorization"), auth_header_value)
}
