//! Session-model integration tests (DB-gated; see `tests/models/users.rs`
//! for the harness rationale — all `#[ignore]`d without PostgreSQL).

use authentication_service::{
    app::App,
    models::sessions::{self, session_data},
    secret_hash,
};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serial_test::serial;
use uuid::Uuid;

/// SEC-A9: `sessions.jid` and the CSRF token in `sessions.data.csrf` are
/// **bearer-equivalent** secrets, so the row at rest must hold only their
/// **hashes** — never the values the client presents. Issue a session with a
/// known plaintext session id + CSRF token, then read the raw row back and
/// assert neither plaintext is stored, that each column holds
/// `secret_hash::hash(plaintext)`, and that `find_by_jid` still resolves the
/// session from the presented plaintext id.
#[tokio::test]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
#[serial]
async fn session_secrets_are_hashed_at_rest() {
    let boot = boot_test::<App>().await.unwrap();

    let sid = "plaintext-session-id-abcdef";
    let csrf = "plaintext-csrf-token-123456";
    let user_pid = Uuid::new_v4();
    let attributes = serde_json::json!({ "access": ["write"] });

    let issued = sessions::Model::issue(
        &boot.app_context.db,
        sid,
        user_pid,
        None,
        session_data(&attributes, csrf),
    )
    .await
    .expect("issue session");

    // Read the raw row straight from the table (no lookup helper in the way).
    let row = sessions::Entity::find_by_id(issued.id)
        .one(&boot.app_context.db)
        .await
        .expect("query session")
        .expect("session row exists");

    // The session id is stored hashed, not in plaintext.
    assert_ne!(row.jid, sid, "the plaintext session id must not be stored");
    assert_eq!(row.jid, secret_hash::hash(sid), "jid is stored as its hash");

    // The CSRF token is stored hashed inside the JSONB payload.
    let stored_csrf = row.data.get("csrf").and_then(serde_json::Value::as_str);
    assert_eq!(stored_csrf, Some(secret_hash::hash(csrf).as_str()));
    assert_ne!(stored_csrf, Some(csrf), "the plaintext csrf must not be stored");

    // A lookup by the *presented plaintext* id still resolves the session.
    let found = sessions::Model::find_by_jid(&boot.app_context.db, sid)
        .await
        .expect("find_by_jid resolves the session from the plaintext id");
    assert_eq!(found.id, issued.id);
}
