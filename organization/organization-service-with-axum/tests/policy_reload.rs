//! ABAC policy hot-reload (authorization-attributes.md §12): an operator
//! edits `ORGANIZATION_ABAC_POLICY_FILE` and the new policy takes effect
//! without a restart, whether reloaded on demand or picked up by the
//! file watcher; a malformed edit falls back to the built-in default
//! rather than leaving the service with no policy.
//!
//! Its own binary: the policy is cached process-wide.

use authentication_verifier::{Action, Claims};
use organization_service::auth::{policy, reload_policy, spawn_policy_watcher};

fn reader() -> Claims {
    serde_json::from_value(serde_json::json!({
        "sub": "11111111-1111-1111-1111-111111111111", "email": "a@example.test", "name": "A",
        "iss": "authentication-service", "aud": "main-x-service", "exp": 9_999_999_999_i64,
        "iat": 1, "sid": "s", "attrs": {}
    }))
    .expect("claims")
}

fn allowed(action: Action) -> bool {
    policy()
        .current()
        .evaluate(&reader(), action, "organization")
        .allowed
}

const DENY_READ: &str = r#"{ "rules": [ { "effect": "deny", "actions": ["read"], "when": {} } ] }"#;
const ALLOW_WRITE: &str =
    r#"{ "rules": [ { "effect": "allow", "actions": ["write"], "when": {} } ] }"#;

#[tokio::test(start_paused = true)]
async fn a_policy_file_edit_takes_effect_without_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("policy.json");
    std::fs::write(&file, DENY_READ).unwrap();
    // SAFETY: single-threaded setup, before the policy is first read.
    unsafe {
        std::env::remove_var("ORGANIZATION_ABAC_POLICY");
        std::env::set_var("ORGANIZATION_ABAC_POLICY_FILE", &file);
    }
    assert!(
        !allowed(Action::Read),
        "the file's deny-read rule is in force"
    );

    // On demand.
    std::fs::write(&file, ALLOW_WRITE).unwrap();
    reload_policy();
    assert!(allowed(Action::Read), "read is default-allowed again");
    assert!(
        allowed(Action::Write),
        "the new allow-write rule is in force"
    );

    // A malformed edit falls back to the default: read allowed, write not.
    std::fs::write(&file, "{ not json").unwrap();
    reload_policy();
    assert!(allowed(Action::Read));
    assert!(!allowed(Action::Write), "the default denies writes");

    // The watcher picks an edit up after its poll interval. Its mtime
    // check reads the real file, so give the edit a later mtime first.
    spawn_policy_watcher();
    tokio::task::yield_now().await;
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&file, DENY_READ).unwrap();
    tokio::time::advance(std::time::Duration::from_secs(16)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert!(
        !allowed(Action::Read),
        "the watcher reloaded the edited file"
    );
}
