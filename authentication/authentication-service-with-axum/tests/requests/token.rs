//! Request tests for `POST /api/auth/token` (spec §5, SEC-A10): the
//! exchange of a browser session for a short-lived PASETO.
//!
//! The endpoint's CSRF decision is unit-tested as `csrf_token_gate`; these
//! drive the handler itself, so the cookie parsing, the session lookup, the
//! CSRF hash comparison and the minted token are each exercised for real.
//! DB-gated like the rest of the request suite.

use authentication_service::{app::App, auth::verify_token, csrf, models::sessions, models::users};
use axa_test::prelude::*;
use axum::http::{HeaderName, HeaderValue};
use serial_test::serial;

use super::prepare_data;

/// A signed-in browser: the two cookie values a magic-link redemption sets.
struct Browser {
    session: String,
    csrf: String,
    user: users::Model,
}

/// The value of cookie `name` from the response's `Set-Cookie` headers.
fn set_cookie(response: &axum_test::TestResponse, name: &str) -> String {
    let prefix = format!("{name}=");
    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|v| v.strip_prefix(&prefix))
        .and_then(|rest| rest.split(';').next())
        .unwrap_or_else(|| panic!("no {name} cookie set"))
        .to_string()
}

/// Sign up and redeem a magic link, keeping the cookies it sets.
async fn sign_in(request: &TestServer, ctx: &axa::AppContext) -> Browser {
    let email = "token@example.com";
    let signup = request
        .post("/api/auth/signup")
        .json(&serde_json::json!({ "email": email, "name": "token" }))
        .await;
    assert_eq!(signup.status_code(), 200);
    let user = users::Model::find_by_email(&ctx.db, email)
        .await
        .expect("signed up");
    let link = prepare_data::issue_magic_link(ctx, user).await;
    let verify = request.get(&format!("/api/auth/magic-link/{link}")).await;
    assert_eq!(verify.status_code(), 200);
    Browser {
        session: set_cookie(&verify, authentication_service::cookie::SESSION_COOKIE),
        csrf: set_cookie(&verify, csrf::CSRF_COOKIE),
        user: users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("still there"),
    }
}

fn header(name: &str, value: &str) -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_bytes(name.as_bytes()).unwrap(),
        HeaderValue::from_str(value).unwrap(),
    )
}

/// `POST /api/auth/token` with the given cookie header and CSRF header.
async fn exchange(
    request: &TestServer,
    cookie: Option<&str>,
    csrf_token: Option<&str>,
) -> axum_test::TestResponse {
    let mut call = request.post("/api/auth/token");
    if let Some(cookie) = cookie {
        let (n, v) = header("cookie", cookie);
        call = call.add_header(n, v);
    }
    if let Some(token) = csrf_token {
        let (n, v) = header(csrf::CSRF_HEADER, token);
        call = call.add_header(n, v);
    }
    call.await
}

fn session_cookie(sid: &str) -> String {
    format!("{}={sid}", authentication_service::cookie::SESSION_COOKIE)
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn a_session_with_its_csrf_token_gets_a_paseto_for_its_user() {
    request::<App, _, _>(|request, ctx| async move {
        let b = sign_in(&request, &ctx).await;
        let response = exchange(&request, Some(&session_cookie(&b.session)), Some(&b.csrf)).await;
        assert_eq!(response.status_code(), 200, "{}", response.text());
        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let token = body["token"].as_str().expect("a token");
        let claims = verify_token(token).expect("the minted token verifies");
        assert_eq!(claims.sub, b.user.pid.to_string());
        assert_eq!(claims.email, b.user.email);
        assert_eq!(claims.sid, b.session, "the token is bound to this session");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn no_session_cookie_is_401() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(exchange(&request, None, None).await.status_code(), 401);
        let other = exchange(&request, Some("theme=dark"), None).await;
        assert_eq!(
            other.status_code(),
            401,
            "an unrelated cookie is no session"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn an_unknown_session_is_401() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = exchange(&request, Some(&session_cookie("not-a-session")), Some("x")).await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

/// SEC-A10: the session carries a CSRF synchroniser token, so a request
/// that does not echo it, or echoes the wrong one, is refused with `403`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn a_missing_or_wrong_csrf_token_is_403() {
    request::<App, _, _>(|request, ctx| async move {
        let b = sign_in(&request, &ctx).await;
        let cookie = session_cookie(&b.session);
        assert_eq!(
            exchange(&request, Some(&cookie), None).await.status_code(),
            403
        );
        assert_eq!(
            exchange(&request, Some(&cookie), Some("forged"))
                .await
                .status_code(),
            403
        );
        // The session cookie's own value is not the CSRF token either.
        assert_eq!(
            exchange(&request, Some(&cookie), Some(&b.session))
                .await
                .status_code(),
            403
        );
    })
    .await;
}

/// A revoked session mints nothing, even with the right CSRF token.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn a_revoked_session_is_401() {
    request::<App, _, _>(|request, ctx| async move {
        let b = sign_in(&request, &ctx).await;
        sessions::Model::revoke_all_for_user(&ctx.db, b.user.pid)
            .await
            .expect("revoked");
        let response = exchange(&request, Some(&session_cookie(&b.session)), Some(&b.csrf)).await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}
