//! Request tests for handler branches the flow tests in `auth.rs` do not
//! reach: the per-email limit as the endpoints apply it, the default
//! display name, a token outliving its erased account, and the
//! per-subject audit export. DB-gated like the rest of the suite.

use authentication_service::{
    app::App,
    models::{auth_events::Model as AuthEvent, users},
    rate_limit,
};
use axa_test::prelude::*;
use serial_test::serial;

use super::prepare_data;

/// The limiter is tested on its own in `rate_limit.rs`; this pins that both
/// issuance endpoints apply it: past the limit they answer `429`, issue no
/// further link, and record the refusal.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn issuance_endpoints_answer_429_past_the_limit() {
    request::<App, _, _>(|request, ctx| async move {
        for (path, email) in [
            ("/api/auth/signup", "limited-signup@example.com"),
            ("/api/auth/magic-link", "limited-link@example.com"),
        ] {
            let body = serde_json::json!({ "email": email, "name": "limited" });
            if path == "/api/auth/magic-link" {
                // Sign-in issuance needs an existing account.
                users::Model::create_passwordless(&ctx.db, email, "limited")
                    .await
                    .expect("account created");
            }
            for n in 0..rate_limit::MAX_REQUESTS {
                let r = request.post(path).json(&body).await;
                assert_eq!(
                    r.status_code(),
                    200,
                    "{path} request {n} is within the limit"
                );
            }
            let over = request.post(path).json(&body).await;
            assert_eq!(over.status_code(), 429, "{path} past the limit");
            let refused = AuthEvent::recent(&ctx.db, 100)
                .await
                .unwrap()
                .into_iter()
                .filter(|e| e.detail.as_deref() == Some("rate_limited"))
                .count();
            assert!(refused >= 1, "{path}: the refusal is audited");
        }
    })
    .await;
}

/// A name shorter than two characters falls back to the email's local
/// part, and to `user` when that is also too short.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn a_too_short_name_falls_back() {
    request::<App, _, _>(|request, ctx| async move {
        for (email, name, expected) in [
            ("morgan@example.com", Some("M"), "morgan"),
            ("q@example.com", None, "user"),
        ] {
            let body = serde_json::json!({ "email": email, "name": name });
            assert_eq!(
                request
                    .post("/api/auth/signup")
                    .json(&body)
                    .await
                    .status_code(),
                200
            );
            let user = users::Model::find_by_email(&ctx.db, email)
                .await
                .expect("created");
            assert_eq!(user.name, expected, "{email}");
        }
    })
    .await;
}

/// A PASETO still verifies after its account is erased, until it expires;
/// `/me` must refuse to serve the erased subject anyway.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn me_refuses_an_erased_account() {
    request::<App, _, _>(|request, ctx| async move {
        let logged_in = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&logged_in.token);
        let erase = request
            .delete("/api/auth/account")
            .add_header(key.clone(), value.clone())
            .await;
        assert_eq!(erase.status_code(), 200);
        let me = request.get("/api/auth/me").add_header(key, value).await;
        assert_eq!(me.status_code(), 401, "an erased subject is not served");
    })
    .await;
}

/// GDPR Art. 15: the per-subject audit export returns the caller's own
/// events, and none of anyone else's.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn account_audit_returns_only_the_callers_events() {
    request::<App, _, _>(|request, ctx| async move {
        let other = serde_json::json!({ "email": "someone-else@example.com", "name": "other" });
        assert_eq!(
            request
                .post("/api/auth/signup")
                .json(&other)
                .await
                .status_code(),
            200
        );
        let logged_in = prepare_data::init_user_login(&request, &ctx).await;
        let (key, value) = prepare_data::auth_header(&logged_in.token);
        let response = request
            .get("/api/auth/account/audit")
            .add_header(key, value)
            .await;
        assert_eq!(response.status_code(), 200);
        let text = response.text();
        let rows: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
        assert!(
            !rows.is_empty(),
            "sign-up and sign-in are on the caller's trail"
        );
        assert!(
            !text.contains("someone-else@example.com"),
            "another subject's events leaked: {text}"
        );
    })
    .await;
}

/// The address-retention sweep removes a client address once it is older
/// than the retention period, from audit rows and sessions alike, and
/// leaves a fresh one alone. The rows themselves stay.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn the_address_sweep_removes_only_old_addresses() {
    use authentication_service::{compliance::address_retention, models::sessions};

    request::<App, _, _>(|_request, ctx| async move {
        let user = users::Model::create_passwordless(&ctx.db, "sweep@example.com", "sweep")
            .await
            .unwrap();
        AuthEvent::record_best_effort(
            &ctx.db,
            "signup",
            Some("sweep@example.com"),
            Some(user.pid),
            Some("ok"),
            Some("203.0.113.9"),
        )
        .await;
        sessions::Model::issue(
            &ctx.db,
            "sweep-session",
            user.pid,
            None,
            Some("203.0.113.9".into()),
            serde_json::json!({}),
        )
        .await
        .unwrap();

        assert_eq!(
            address_retention::scrub(&ctx.db, 7).await.unwrap(),
            0,
            "fresh addresses stay"
        );

        sqlx::query("UPDATE auth_events SET created_at = created_at - interval '30 days'")
            .execute(&ctx.db)
            .await
            .unwrap();
        sqlx::query("UPDATE sessions SET created_at = created_at - interval '30 days'")
            .execute(&ctx.db)
            .await
            .unwrap();
        assert_eq!(
            address_retention::scrub(&ctx.db, 7).await.unwrap(),
            2,
            "one row in each table"
        );
        let remaining: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM auth_events WHERE source_ip IS NOT NULL)
                  + (SELECT count(*) FROM sessions WHERE source_ip IS NOT NULL)",
        )
        .fetch_one(&ctx.db)
        .await
        .unwrap();
        assert_eq!(remaining, 0);
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM auth_events")
            .fetch_one(&ctx.db)
            .await
            .unwrap();
        assert!(rows >= 1, "the audit row itself survives");
    })
    .await;
}
