//! Shared helpers for request tests: drive the passwordless magic-link
//! flow end-to-end to obtain a signed-in user + RS256 bearer token.

use authentication_service::{models::users, views::auth::LoginResponse};
use axum::http::{HeaderName, HeaderValue};
use loco_rs::{app::AppContext, TestServer};

const USER_EMAIL: &str = "test@loco.com";
const USER_NAME: &str = "loco";

pub struct LoggedInUser {
    pub user: users::Model,
    pub token: String,
}

/// Sign up a fresh account, pull the magic-link token from the
/// database (in dev/test there is no SMTP — the link is logged), and
/// redeem it to obtain an RS256 access token.
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
    let magic_link_token = user
        .magic_link_token
        .clone()
        .expect("signup should have issued a magic link token");

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

pub fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let auth_header_value = HeaderValue::from_str(&format!("Bearer {}", &token)).unwrap();

    (HeaderName::from_static("authorization"), auth_header_value)
}
