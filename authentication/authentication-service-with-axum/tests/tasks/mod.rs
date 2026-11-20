//! Task-layer integration tests: the `user_attributes` CLI task, run the
//! way `cargo run -- task user_attributes …` runs it. DB-gated.
//!
//! The task's parsing and the attribute arithmetic are unit-tested in
//! `src/tasks/attributes.rs`; these pin what only a run shows: the change
//! is persisted, the user's sessions are revoked (SEC-A8), the assignment
//! is audited, and a bad target or argument is refused before any write.

use authentication_service::{
    app::App,
    models::{
        auth_events::{self, Model as AuthEvent},
        sessions, users,
    },
    tasks::attributes::UserAttributes,
};
use axa::task::{Task, Vars};
use axa_test::prelude::*;
use serial_test::serial;

fn vars(pairs: &[(&str, &str)]) -> Vars {
    Vars::from_cli_args(
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect(),
    )
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn set_persists_revokes_sessions_and_audits() {
    let boot = boot_test::<App>().await.expect("boot");
    let ctx = &boot.app_context;
    let user = users::Model::create_passwordless(&ctx.db, "ops@example.com", "ops")
        .await
        .expect("user");
    let session = sessions::Model::issue(
        &ctx.db,
        "task-test-session",
        user.pid,
        None,
        None,
        serde_json::json!({}),
    )
    .await
    .expect("session");
    assert!(session.is_active());

    UserAttributes
        .run(
            ctx,
            &vars(&[
                ("email", "ops@example.com"),
                ("op", "set"),
                ("key", "access"),
                ("values", "write"),
            ]),
        )
        .await
        .expect("set succeeds");

    let after = users::Model::find_by_email(&ctx.db, "ops@example.com")
        .await
        .unwrap();
    assert_eq!(
        after.attrs().get("access").map(Vec::as_slice),
        Some(&["write".to_string()][..])
    );
    let still = sessions::Model::find_all_by_user_pid(&ctx.db, user.pid)
        .await
        .unwrap();
    assert!(
        still.iter().all(|s| !s.is_active()),
        "SEC-A8: a change must revoke the user's sessions"
    );
    let audited = AuthEvent::recent(&ctx.db, 50)
        .await
        .unwrap()
        .into_iter()
        .any(|e| e.event == auth_events::ATTRIBUTES_ASSIGNED && e.user_pid == Some(user.pid));
    assert!(audited, "the assignment is audited");
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn unset_and_clear_change_what_is_stored_and_show_changes_nothing() {
    let boot = boot_test::<App>().await.expect("boot");
    let ctx = &boot.app_context;
    let user = users::Model::create_passwordless(&ctx.db, "ops2@example.com", "ops")
        .await
        .expect("user");
    let pid = user.pid.to_string();
    let run = |pairs: Vec<(&'static str, String)>| {
        let v = Vars::from_cli_args(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect());
        async move { UserAttributes.run(ctx, &v).await }
    };
    run(vec![
        ("pid", pid.clone()),
        ("op", "set".into()),
        ("key", "dept".into()),
        ("values", "care,oncology".into()),
    ])
    .await
    .unwrap();
    run(vec![
        ("pid", pid.clone()),
        ("op", "set".into()),
        ("key", "access".into()),
        ("values", "write".into()),
    ])
    .await
    .unwrap();
    run(vec![
        ("pid", pid.clone()),
        ("op", "unset".into()),
        ("key", "dept".into()),
    ])
    .await
    .unwrap();
    let attrs = users::Model::find_by_pid(&ctx.db, &pid)
        .await
        .unwrap()
        .attrs();
    assert!(!attrs.contains_key("dept") && attrs.contains_key("access"));

    run(vec![("pid", pid.clone())])
        .await
        .expect("show is the default op");
    assert_eq!(
        users::Model::find_by_pid(&ctx.db, &pid)
            .await
            .unwrap()
            .attrs(),
        attrs
    );

    run(vec![("pid", pid.clone()), ("op", "clear".into())])
        .await
        .unwrap();
    assert!(
        users::Model::find_by_pid(&ctx.db, &pid)
            .await
            .unwrap()
            .attrs()
            .is_empty()
    );
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with: cargo test -- --ignored"]
async fn bad_targets_and_arguments_are_refused() {
    let boot = boot_test::<App>().await.expect("boot");
    let ctx = &boot.app_context;
    users::Model::create_passwordless(&ctx.db, "ops3@example.com", "ops")
        .await
        .expect("user");
    for (why, pairs) in [
        ("no target", vec![("op", "show")]),
        (
            "both targets",
            vec![("email", "ops3@example.com"), ("pid", "x")],
        ),
        ("unknown user", vec![("email", "nobody@example.com")]),
        (
            "unknown op",
            vec![("email", "ops3@example.com"), ("op", "grant")],
        ),
        (
            "set without key",
            vec![("email", "ops3@example.com"), ("op", "set")],
        ),
    ] {
        assert!(
            UserAttributes.run(ctx, &vars(&pairs)).await.is_err(),
            "{why}"
        );
    }
    let untouched = users::Model::find_by_email(&ctx.db, "ops3@example.com")
        .await
        .unwrap();
    assert!(
        untouched.attrs().is_empty(),
        "a refused command writes nothing"
    );
}
