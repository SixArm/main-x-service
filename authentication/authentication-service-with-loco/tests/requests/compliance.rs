//! Request tests for the audit-integrity endpoint
//! (`GET /api/compliance/audit/verify`, spec §6.13 / §16, PRO-P23).
//!
//! Pins the decided behaviour: the endpoint requires a valid PASETO
//! bearer (`401` without one) but is **not** admin-gated (any
//! authenticated caller may call it — the report carries no PII, and
//! the bearer requirement exists to remove anonymous-internet abuse of
//! the recomputation cost, not to protect a secret).
//!
//! The DB-backed flow tests boot the loco app and need the PostgreSQL
//! instance from `config/test.yaml`; they are `#[ignore]`d so a checkout
//! without Postgres keeps `cargo test` green. Run them with:
//!
//! ```text
//! cargo test -- --ignored
//! ```

use authentication_service::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

use super::prepare_data::{auth_header, init_user_login};

/// No bearer token at all is rejected with `401` (the `AuthUser`
/// extractor) — the endpoint is not reachable unauthenticated.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn missing_token_is_unauthorized() {
    request::<App, _, _>(|request, _ctx| async move {
        let res = request.get("/api/compliance/audit/verify").await;
        assert_eq!(res.status_code(), 401);
    })
    .await;
}

/// A plain, non-admin authenticated caller IS allowed — this endpoint is
/// deliberately not admin-gated, unlike `/api/auth/audit/recent`, since
/// its report carries no PII (row counts and row ids only).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn any_authenticated_caller_is_allowed() {
    request::<App, _, _>(|request, ctx| async move {
        let logged_in = init_user_login(&request, &ctx).await;
        let (k, v) = auth_header(&logged_in.token);
        let res = request
            .get("/api/compliance/audit/verify")
            .add_header(k, v)
            .await;
        assert_eq!(
            res.status_code(),
            200,
            "a plain authenticated caller (no access=admin) should be allowed"
        );
        let body: serde_json::Value = serde_json::from_str(&res.text()).unwrap();
        assert!(body["verified"].is_boolean());
        assert!(body["caveat"].is_string());
    })
    .await;
}
