use case_tracking_with_loco::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

const KNOWN_EMAIL: &str = "tester@example.nhs.uk";

#[tokio::test]
#[serial]
async fn request_returns_dev_magic_link_for_known_email() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/auth/request")
            .json(&serde_json::json!({ "email": KNOWN_EMAIL }))
            .await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["sent"], true);
        let link = body["magic_link"].as_str().unwrap();
        assert!(link.contains("/auth/callback?token="));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn request_for_unknown_email_returns_ok_without_link() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/auth/request")
            .json(&serde_json::json!({ "email": "stranger@example.com" }))
            .await;
        // 200 either way so the endpoint can't enumerate valid emails.
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["sent"], true);
        assert!(body.get("magic_link").is_none() || body["magic_link"].is_null());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn request_with_blank_email_returns_422() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/auth/request")
            .json(&serde_json::json!({ "email": "  " }))
            .await;
        assert_eq!(response.status_code(), 422);
        let body: serde_json::Value = response.json();
        assert!(body["errors"]["email"].is_string());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn verify_with_invalid_token_returns_401() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/auth/verify")
            .json(&serde_json::json!({ "token": "not-a-real-token" }))
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn me_without_session_returns_401() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request.get("/api/auth/me").await;
        assert_eq!(response.status_code(), 401);
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"], "Authentication required");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn request_verify_me_round_trip() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;

        // 1. Ask for a link and read the dev-exposed token.
        let requested = request
            .post("/api/auth/request")
            .json(&serde_json::json!({ "email": KNOWN_EMAIL }))
            .await;
        let body: serde_json::Value = requested.json();
        let link = body["magic_link"].as_str().unwrap().to_string();
        let token = link.split("token=").nth(1).unwrap().to_string();

        // 2. Exchange it for a session.
        let verified = request
            .post("/api/auth/verify")
            .json(&serde_json::json!({ "token": token }))
            .await;
        assert_eq!(verified.status_code(), 200);
        let vbody: serde_json::Value = verified.json();
        assert_eq!(vbody["user"]["email"], KNOWN_EMAIL);
        assert_eq!(vbody["user"]["name"], "Test User");

        let set_cookie = verified
            .headers()
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(set_cookie.contains("cts_session="));
        assert!(set_cookie.contains("HttpOnly"));
        let session = super::session_token_from_set_cookie(&set_cookie);

        // 3. Present the session as a Bearer token to /me.
        let me = request
            .get("/api/auth/me")
            .authorization_bearer(session)
            .await;
        assert_eq!(me.status_code(), 200);
        let mbody: serde_json::Value = me.json();
        assert_eq!(mbody["user"]["email"], KNOWN_EMAIL);
    })
    .await;
}
