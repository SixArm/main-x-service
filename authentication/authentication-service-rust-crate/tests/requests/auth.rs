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

/// Pins that signup persists the user and issues a magic-link token with
/// an expiry, returning 200.
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

/// Pins anti-enumeration on signup: an existing email is indistinguishable
/// from a fresh one — same 200, and it still receives a fresh link.
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

/// Pins that requesting a magic link for a known account issues a token
/// and returns 200 (the sign-in request path).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn can_request_magic_link_for_existing_account() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let payload = serde_json::json!({ "email": "user1@example.com" });
        let response = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(
            response.status_code(),
            200,
            "Magic link request should succeed"
        );

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

/// Pins anti-enumeration on the sign-in request: an unknown email gets
/// the same 200 as a known one (no account-existence leak).
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

/// Pins redemption: a valid token yields a verifying RS256 access token,
/// marks the email verified, and is single-use (second redeem → 401).
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

/// Pins that an unknown/invalid magic-link token is rejected with 401
/// (same status as expired/consumed — no distinguishing leak).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn invalid_magic_link_token_is_rejected() {
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let response = request.get("/api/auth/magic-link/invalid-token").await;
        assert_eq!(
            response.status_code(),
            401,
            "Invalid token must be rejected"
        );
    })
    .await;
}

/// Pins `/me` with a valid bearer token returns the current user's
/// public fields (pid / email / name).
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
        assert_eq!(
            response.status_code(),
            200,
            "/me should succeed with a bearer token"
        );

        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(body["pid"], logged_in.user.pid.to_string());
        assert_eq!(body["email"], logged_in.user.email);
        assert_eq!(body["name"], logged_in.user.name);
    })
    .await;
}

/// Pins that `/me` without a bearer token is 401 (the route is gated).
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

/// Pins local revocation: after signout the JWT still verifies
/// cryptographically, but `/me` rejects the revoked session with 401.
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
        assert_eq!(
            me.status_code(),
            401,
            "Revoked session must be rejected by /me"
        );
    })
    .await;
}

/// Pins that the public JWKS endpoint serves the RSA signing key and the
/// published `kid` matches the one stamped into token headers.
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

/// Pins the per-email rate limit end-to-end: the first `MAX_REQUESTS`
/// issuance calls stay 200, and the next is throttled with 429 — without
/// leaking account existence.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn magic_link_issuance_is_rate_limited() {
    use authentication_service::rate_limit;

    request::<App, _, _>(|request, _ctx| async move {
        // The limiter store is process-wide; start from a clean slate so
        // the per-email window is deterministic for this email.
        rate_limit::reset();

        let email = "rate-limit@example.com";
        let payload = serde_json::json!({ "email": email });

        // The first MAX_REQUESTS are accepted with the always-200
        // anti-enumeration shape, even though the email is unknown.
        for i in 0..rate_limit::MAX_REQUESTS {
            let response = request.post("/api/auth/magic-link").json(&payload).await;
            assert_eq!(
                response.status_code(),
                200,
                "request {i} (within the quota) must stay 200"
            );
        }

        // The N+1th request inside the window is throttled with 429.
        let throttled = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(
            throttled.status_code(),
            429,
            "the (MAX_REQUESTS+1)th request must be rate-limited"
        );

        rate_limit::reset();
    })
    .await;
}

/// Pins the audit trail: signup and an unknown-email request both write
/// `auth_events` rows (the latter with the `unknown_email` outcome), the
/// rows never carry token-like material, and `/audit/recent` surfaces them.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn auth_events_are_recorded_and_queryable() {
    use authentication_service::{models::auth_events::Model as AuthEvent, rate_limit};

    request::<App, _, _>(|request, ctx| async move {
        rate_limit::reset();

        // A signup writes a `signup` audit row (created/existing).
        let signup_email = "audit-signup@example.com";
        let signup = request
            .post("/api/auth/signup")
            .json(&serde_json::json!({ "email": signup_email, "name": "Audit" }))
            .await;
        assert_eq!(signup.status_code(), 200);

        // A magic-link request for an unknown email writes a
        // `magic_link_requested` row with the `unknown_email` outcome —
        // even though the HTTP response is the same 200 as a known one.
        let unknown_email = "audit-unknown@example.com";
        let magic = request
            .post("/api/auth/magic-link")
            .json(&serde_json::json!({ "email": unknown_email }))
            .await;
        assert_eq!(magic.status_code(), 200);

        // The model sees both rows.
        let rows = AuthEvent::recent(&ctx.db, 100)
            .await
            .expect("recent auth events should query");
        assert!(
            rows.iter()
                .any(|r| r.event == "signup" && r.email.as_deref() == Some(signup_email)),
            "expected a signup auth event for {signup_email}; got {rows:?}"
        );
        assert!(
            rows.iter().any(|r| r.event == "magic_link_requested"
                && r.email.as_deref() == Some(unknown_email)
                && r.detail.as_deref() == Some("unknown_email")),
            "expected an unknown_email magic_link_requested event; got {rows:?}"
        );
        // No row ever carries a token or secret column.
        assert!(
            !rows.iter().any(|r| r
                .detail
                .as_deref()
                .is_some_and(|d| d.contains("token") && d.len() > 32)),
            "audit detail must not leak token-like material"
        );

        // The read endpoint surfaces them, newest first.
        let recent = request.get("/api/auth/audit/recent").await;
        assert_eq!(recent.status_code(), 200, "/audit/recent should be public");
        let body: serde_json::Value = serde_json::from_str(&recent.text()).unwrap();
        let events = body.as_array().expect("audit/recent returns an array");
        assert!(
            events
                .iter()
                .any(|e| e["event"] == "signup" && e["email"] == signup_email),
            "audit/recent should include the signup event"
        );
        assert!(
            events
                .iter()
                .any(|e| e["event"] == "magic_link_requested" && e["detail"] == "unknown_email"),
            "audit/recent should include the unknown_email magic-link event"
        );

        rate_limit::reset();
    })
    .await;
}

/// Pins the GDPR Art. 15 export: it returns the subject's user row,
/// sessions, and audit events — and never any password / api key / token.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn account_export_returns_the_callers_data() {
    // GDPR Art. 15 (right of access): the export gathers the subject's
    // users row, sessions, and auth_events.
    request::<App, _, _>(|request, ctx| async move {
        let logged_in = prepare_data::init_user_login(&request, &ctx).await;
        let (auth_key, auth_value) = prepare_data::auth_header(&logged_in.token);

        let response = request
            .get("/api/auth/account/export")
            .add_header(auth_key, auth_value)
            .await;
        assert_eq!(
            response.status_code(),
            200,
            "export should succeed with a bearer token"
        );

        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        assert_eq!(body["user"]["pid"], logged_in.user.pid.to_string());
        assert_eq!(body["user"]["email"], logged_in.user.email);
        // At least the redemption session is present.
        assert!(
            body["sessions"].as_array().is_some_and(|s| !s.is_empty()),
            "export should include the subject's sessions"
        );
        // The audit trail (signup + redeem) is present.
        assert!(
            body["auth_events"]
                .as_array()
                .is_some_and(|e| !e.is_empty()),
            "export should include the subject's auth events"
        );
        // No credentials/secrets ever appear.
        let raw = response.text();
        assert!(
            !raw.contains("password"),
            "export must not expose the password"
        );
        assert!(
            !raw.contains("api_key"),
            "export must not expose the api key"
        );
        assert!(
            !raw.contains("\"token\""),
            "export must not expose any token"
        );
    })
    .await;
}

/// Pins that the GDPR account routes (export / per-subject audit /
/// erasure) all require a bearer token (401 without one).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn account_export_and_delete_require_a_bearer_token() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(
            request.get("/api/auth/account/export").await.status_code(),
            401,
            "export requires a bearer token"
        );
        assert_eq!(
            request.get("/api/auth/account/audit").await.status_code(),
            401,
            "per-subject audit requires a bearer token"
        );
        assert_eq!(
            request.delete("/api/auth/account").await.status_code(),
            401,
            "erasure requires a bearer token"
        );
    })
    .await;
}

/// Pins GDPR Art. 17 erasure end-to-end: the row survives but is
/// soft-deleted + anonymised (tombstone email/name), the original email
/// no longer resolves, all sessions are revoked, an `account_erased`
/// audit row is written, and post-erasure `/me` + export return 401.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn account_erasure_soft_deletes_anonymises_revokes_and_audits() {
    // GDPR Art. 17 (right to erasure): soft-delete + anonymise + revoke
    // sessions + audit; after erasure /me and export are gone.
    use authentication_service::models::{auth_events::Model as AuthEvent, sessions, users};

    request::<App, _, _>(|request, ctx| async move {
        let logged_in = prepare_data::init_user_login(&request, &ctx).await;
        let pid = logged_in.user.pid;
        let original_email = logged_in.user.email.clone();
        let (auth_key, auth_value) = prepare_data::auth_header(&logged_in.token);

        let erase = request
            .delete("/api/auth/account")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(erase.status_code(), 200, "erasure should succeed");

        // The row survives but is soft-deleted + anonymised.
        let erased = users::Model::find_by_pid(&ctx.db, &pid.to_string())
            .await
            .expect("the row must survive erasure for audit integrity");
        assert!(erased.deleted_at.is_some(), "deleted_at must be stamped");
        assert_ne!(erased.email, original_email, "email must be anonymised");
        assert!(
            erased.email.ends_with("@invalid"),
            "email must be tombstoned"
        );
        assert_eq!(erased.name, "deleted user", "name must be tombstoned");
        // The original email no longer resolves.
        assert!(
            users::Model::find_by_email(&ctx.db, &original_email)
                .await
                .is_err(),
            "the original email must no longer resolve to the account"
        );

        // All sessions are revoked.
        let sessions = sessions::Model::find_all_by_user_pid(&ctx.db, pid)
            .await
            .expect("sessions should query");
        assert!(
            sessions.iter().all(|s| !s.is_active()),
            "every session must be revoked after erasure"
        );

        // An account_erased audit row was written.
        let events = AuthEvent::for_subject(&ctx.db, pid, &original_email)
            .await
            .expect("subject audit should query");
        assert!(
            events.iter().any(|e| e.event == "account_erased"),
            "an account_erased audit row must be recorded; got {events:?}"
        );

        // Post-erasure: /me and export treat the subject as gone (401),
        // even though the bearer token still verifies cryptographically.
        let me = request
            .get("/api/auth/me")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(me.status_code(), 401, "/me must reject an erased account");
        let export = request
            .get("/api/auth/account/export")
            .add_header(auth_key, auth_value)
            .await;
        assert_eq!(
            export.status_code(),
            401,
            "export must reject an erased account"
        );
    })
    .await;
}

// ---------------------------------------------------------------------------
// DB-free contract assertions (always run).
// ---------------------------------------------------------------------------

/// Pins (DB-free) that the auth route table is mounted under `/api/auth`
/// and exposes every expected path, and that the JWKS route is mounted
/// under `/.well-known/jwks.json`.
#[test]
fn route_table_covers_the_magic_link_surface() {
    let routes = authentication_service::controllers::auth::routes();
    assert_eq!(routes.prefix.as_deref(), Some("/api/auth"));
    let uris: Vec<&str> = routes.handlers.iter().map(|h| h.uri.as_str()).collect();
    for expected in [
        "/signup",
        "/magic-link",
        "/magic-link/{token}",
        "/me",
        "/signout",
        "/audit/recent",
        "/account/export",
        "/account/audit",
        "/account",
    ] {
        assert!(
            uris.contains(&expected),
            "missing route {expected}; have {uris:?}"
        );
    }

    let jwks = authentication_service::controllers::jwks::routes();
    assert_eq!(jwks.prefix.as_deref(), Some("/.well-known"));
    assert!(jwks.handlers.iter().any(|h| h.uri == "/jwks.json"));
}

/// Pins (DB-free) that the docs routes expose the `OpenAPI` JSON and the
/// Swagger UI page.
#[test]
fn route_table_covers_the_api_docs_surface() {
    let docs = authentication_service::controllers::docs::routes();
    let uris: Vec<&str> = docs.handlers.iter().map(|h| h.uri.as_str()).collect();
    for expected in ["/api-docs/openapi.json", "/swagger-ui"] {
        assert!(
            uris.contains(&expected),
            "missing docs route {expected}; have {uris:?}"
        );
    }
}

/// Pins (DB-free) the `SignupParams` deserialization contract: `name` is
/// optional, but `email` is required.
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

/// Pins (DB-free) that `SignupParams` accepts an optional `locale` field.
#[test]
fn signup_params_accept_an_optional_locale() {
    use authentication_service::controllers::auth::SignupParams;

    let with_locale: SignupParams =
        serde_json::from_value(serde_json::json!({"email": "a@example.com", "locale": "cy"}))
            .expect("email + locale should deserialize");
    assert_eq!(with_locale.locale.as_deref(), Some("cy"));

    let without_locale: SignupParams =
        serde_json::from_value(serde_json::json!({"email": "a@example.com"}))
            .expect("locale should be optional");
    assert!(without_locale.locale.is_none());
}

/// Pins (DB-free) that `MagicLinkParams` accepts an optional `locale` field.
#[test]
fn magic_link_params_accept_an_optional_locale() {
    use authentication_service::controllers::auth::MagicLinkParams;

    let with_locale: MagicLinkParams =
        serde_json::from_value(serde_json::json!({"email": "a@example.com", "locale": "cy"}))
            .expect("email + locale should deserialize");
    assert_eq!(with_locale.locale.as_deref(), Some("cy"));

    let without_locale: MagicLinkParams =
        serde_json::from_value(serde_json::json!({"email": "a@example.com"}))
            .expect("locale should be optional");
    assert!(without_locale.locale.is_none());
}

/// Anti-enumeration is locale-independent: passing an unknown / Welsh /
/// absent `locale` must not change the always-`200` response shape. The
/// email language differs (covered by the un-gated `i18n` unit tests),
/// but the wire response does not.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn signup_locale_does_not_change_the_response_shape() {
    request::<App, _, _>(|request, _ctx| async move {
        for locale in [
            serde_json::Value::Null,
            serde_json::json!("cy"),
            serde_json::json!("fr"),
            serde_json::json!("not-a-locale"),
        ] {
            let payload = serde_json::json!({
                "email": "locale-probe@loco.com",
                "name": "loco",
                "locale": locale,
            });
            let response = request.post("/api/auth/signup").json(&payload).await;
            assert_eq!(
                response.status_code(),
                200,
                "signup must be 200 regardless of locale {locale:?}"
            );
        }
    })
    .await;
}
