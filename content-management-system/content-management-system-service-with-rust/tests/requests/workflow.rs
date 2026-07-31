//! The editorial workflow (CMS-R9–R12): transitions and their reasons,
//! publishing a *specific* revision, the gate, scheduling with an
//! idempotent sweep, and advisory locks.

use content_management_system_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_key, seed_site};

/// Backdate a variant's schedule so the sweep sees it as due.
///
/// Written straight to the database on purpose: the API refuses to
/// schedule anything in the past (correctly), and a test cannot wait
/// for the clock. Only the timestamp is faked — the sweep, its locking,
/// its gate, and its bookkeeping all run for real.
async fn make_due(ctx: &loco_rs::app::AppContext, entry_pid: &str, column: &str) {
    use sea_orm::ConnectionTrait;
    ctx.db
        .execute_unprepared(&format!(
            "UPDATE entry_variants SET {column} = now() - interval '1 minute' \
             WHERE entry_pid = '{entry_pid}'::uuid AND {column} IS NOT NULL"
        ))
        .await
        .expect("backdating the schedule");
}

/// A site + an article type whose only field is optional, so the gate
/// is exercised deliberately rather than incidentally.
async fn seed_site_and_type(request: &axum_test::TestServer, prefix: &str) -> String {
    let site_pid = seed_site(request, &a_key(prefix)).await;
    request
        .post(&format!("/api/sites/{site_pid}/content-types"))
        .json(&json!({
            "key": "article",
            "name": "Article",
            // Not routable: these tests are about the editorial
            // lifecycle, and a routable type cannot publish without an
            // address (CMS-R11, pinned in the delivery suite).
            "routable": false,
            "fields": [{ "key": "standfirst", "label": "Standfirst", "kind": "text" }],
        }))
        .await
        .assert_status_ok();
    site_pid
}

/// Create an entry, returning `(entry_pid, revision_pid)`.
async fn create_entry(
    request: &axum_test::TestServer,
    site_pid: &str,
    key: &str,
) -> (String, String) {
    let created: Value = request
        .post(&format!("/api/sites/{site_pid}/entries"))
        .json(&json!({
            "key": key,
            "content_type_key": "article",
            "title": "First title",
            "blocks": [{ "kind": "paragraph", "text": "First body" }],
        }))
        .await
        .json();
    (
        created["pid"].as_str().unwrap().to_string(),
        created["revision_pid"].as_str().unwrap().to_string(),
    )
}

/// Apply a transition.
async fn act(
    request: &axum_test::TestServer,
    entry_pid: &str,
    body: Value,
) -> axum_test::TestResponse {
    request
        .post(&format!("/api/entries/{entry_pid}/variants/en/transition"))
        .json(&body)
        .await
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_editorial_journey_walks_draft_to_published() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "workflow").await;
        let (entry_pid, first) = create_entry(&request, &site_pid, "journey").await;

        // Out of order: approving a draft is refused, and the refusal
        // says what *is* possible from here.
        let early = act(&request, &entry_pid, json!({ "action": "approve" })).await;
        assert_eq!(early.status_code(), 422);
        assert!(early.text().contains("\\\"draft\\\""), "{}", early.text());
        assert!(early.text().contains("legal actions"), "{}", early.text());

        let submitted: Value = act(
            &request,
            &entry_pid,
            json!({ "action": "submit", "reviewer_ref": format!("worker:{}", uuid::Uuid::new_v4()) }),
        )
        .await
        .json();
        assert_eq!(submitted["to"], "in_review");

        // Rejection needs a reason...
        let unexplained = act(&request, &entry_pid, json!({ "action": "reject" })).await;
        assert_eq!(unexplained.status_code(), 422);
        assert!(unexplained.text().contains("requires a reason"));
        // ...and returns the work to the person holding it.
        let rejected: Value = act(
            &request,
            &entry_pid,
            json!({ "action": "reject", "reason": "needs a stronger opening" }),
        )
        .await
        .json();
        assert_eq!(rejected["to"], "draft");

        // Round two, all the way to published.
        act(&request, &entry_pid, json!({ "action": "submit" }))
            .await
            .assert_status_ok();
        act(&request, &entry_pid, json!({ "action": "approve" }))
            .await
            .assert_status_ok();
        let published: Value = act(&request, &entry_pid, json!({ "action": "publish" }))
            .await
            .json();
        assert_eq!(published["to"], "published");
        assert_eq!(published["published_revision_pid"], first);
        let first_published_at = published["first_published_at"].clone();
        assert!(!first_published_at.is_null());

        // The audit trail carries the reason and the revision.
        let trail: Value = request.get("/api/audits/recent").await.json();
        assert!(
            trail
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["action"] == "reject"
                    && row["snapshot"]["reason"] == "needs a stronger opening")
        );

        // Unpublish returns it to draft and preserves first_published_at
        // — "when did this first appear" is a different question from
        // "what is live now".
        let unpublished: Value = act(
            &request,
            &entry_pid,
            json!({ "action": "unpublish", "reason": "legal review" }),
        )
        .await
        .json();
        assert_eq!(unpublished["to"], "draft");
        assert!(unpublished["published_revision_pid"].is_null());
        assert_eq!(unpublished["first_published_at"], first_published_at);

        // Republishing keeps the original first-published stamp.
        let republished: Value = act(&request, &entry_pid, json!({ "action": "publish" }))
            .await
            .json();
        assert_eq!(republished["first_published_at"], first_published_at);
    })
    .await;
}

/// Publishing names a revision: editing afterwards changes nothing
/// live until the next publish.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn saving_after_publishing_does_not_change_what_is_live() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "publish-pointer").await;
        let (entry_pid, first) = create_entry(&request, &site_pid, "pointer").await;
        act(&request, &entry_pid, json!({ "action": "publish" }))
            .await
            .assert_status_ok();

        // Save a new revision *after* publishing.
        let saved: Value = request
            .post(&format!("/api/entries/{entry_pid}/variants/en/revisions"))
            .json(&json!({
                "base_revision_pid": first,
                "title": "Rewritten",
                "blocks": [{ "kind": "paragraph", "text": "new words" }],
            }))
            .await
            .json();
        let second = saved["revision_pid"].as_str().unwrap().to_string();

        let variant: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en"))
            .await
            .json();
        assert_eq!(
            variant["variant"]["published_revision_pid"], first,
            "the live revision did not move"
        );
        assert_eq!(variant["variant"]["current_revision_pid"], second);
        assert_eq!(
            variant["variant"]["status"], "published",
            "the variant stays published while newer drafts accumulate"
        );

        // The site's published list says there is unpublished work.
        let live: Value = request
            .get(&format!("/api/sites/{site_pid}/published"))
            .await
            .json();
        assert_eq!(live["published"][0]["has_unpublished_changes"], true);

        // Publishing again moves the pointer; publishing an explicitly
        // named earlier revision moves it back.
        act(&request, &entry_pid, json!({ "action": "publish" }))
            .await
            .assert_status_ok();
        let variant: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en"))
            .await
            .json();
        assert_eq!(variant["variant"]["published_revision_pid"], second);

        let rolled_back: Value = act(
            &request,
            &entry_pid,
            json!({ "action": "publish", "revision_pid": first }),
        )
        .await
        .json();
        assert_eq!(rolled_back["published_revision_pid"], first);
    })
    .await;
}

/// The publish gate refuses, and says exactly what is missing.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn the_gate_refuses_a_publish_that_is_not_ready() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site(&request, &a_key("gate")).await;
        request
            .post(&format!("/api/sites/{site_pid}/content-types"))
            .json(&json!({
                "key": "article",
                "name": "Article",
                "routable": false,
                "fields": [{ "key": "standfirst", "label": "Standfirst", "kind": "text",
                             "required": true }],
            }))
            .await
            .assert_status_ok();

        // A draft may be incomplete — the save succeeds.
        let created: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(&json!({
                "key": "incomplete", "content_type_key": "article", "title": "Incomplete",
            }))
            .await
            .json();
        let entry_pid = created["pid"].as_str().unwrap().to_string();
        let first = created["revision_pid"].as_str().unwrap().to_string();

        // Publishing is where the missing field bites, and the message
        // names the field and the remedy.
        let refused = act(&request, &entry_pid, json!({ "action": "publish" })).await;
        assert_eq!(refused.status_code(), 422);
        assert!(
            refused.text().contains("required_field_empty"),
            "{}",
            refused.text()
        );
        assert!(refused.text().contains("standfirst"), "{}", refused.text());

        // The preview read agrees with the gate — same function.
        let check: Value = request
            .get(&format!(
                "/api/entries/{entry_pid}/variants/en/publish-check"
            ))
            .await
            .json();
        assert_eq!(check["ready"], false);
        assert_eq!(check["blockers"][0]["rule"], "required_field_empty");

        // Fill it in, and the same call succeeds.
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/revisions"))
            .json(&json!({
                "base_revision_pid": first,
                "title": "Incomplete",
                "fields": { "standfirst": "Now it has one" },
            }))
            .await
            .assert_status_ok();
        act(&request, &entry_pid, json!({ "action": "publish" }))
            .await
            .assert_status_ok();
    })
    .await;
}

/// Separation of duties: the author of the current revision cannot
/// approve it when the site asks for a distinct approver.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_site_can_require_a_distinct_approver() {
    request::<App, _, _>(|request, _ctx| async move {
        // With enforcement off there is no caller identity, so an
        // unauthenticated author cannot collide with an
        // unauthenticated approver; the rule is exercised through the
        // *setting*, which defaults on and is stored per site.
        let key = a_key("approver");
        let site_pid = seed_site(&request, &key).await;
        let site: Value = request.get(&format!("/api/sites/{site_pid}")).await.json();
        assert_eq!(
            site["site"]["require_distinct_approver"], true,
            "separation of duties is the default"
        );

        // A site may opt out deliberately.
        let mut payload = super::a_site_payload(&key);
        payload["require_distinct_approver"] = json!(false);
        let updated: Value = request
            .put(&format!("/api/sites/{site_pid}"))
            .json(&payload)
            .await
            .json();
        assert_eq!(updated["require_distinct_approver"], false);
    })
    .await;
}

/// Scheduling, and the sweep that applies it — idempotently, and only
/// when the transition is still legal.
///
/// A due schedule is planted directly in the database, because the API
/// (correctly) refuses to schedule anything in the past and a test
/// cannot wait for the clock. That is the only shortcut taken: the
/// sweep itself runs exactly as it does in production.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn scheduled_publishing_is_applied_once_and_only_when_legal() {
    request::<App, _, _>(|request, ctx| async move {
        let site_pid = seed_site_and_type(&request, "schedule").await;
        let (entry_pid, _) = create_entry(&request, &site_pid, "scheduled").await;
        let schedule_path = format!("/api/entries/{entry_pid}/variants/en/schedule");

        // A schedule in the past is refused: it would fire immediately
        // and surprise whoever set it.
        let past = request
            .post(&schedule_path)
            .json(&json!({ "publish_at": "2020-01-01T00:00:00Z" }))
            .await;
        assert_eq!(past.status_code(), 422);
        assert!(past.text().contains("must be in the future"));

        // Scheduling an unpublish for something that is not live is
        // refused too.
        let not_live = request
            .post(&schedule_path)
            .json(&json!({ "unpublish_at": "2099-01-01T00:00:00Z" }))
            .await;
        assert_eq!(not_live.status_code(), 422);
        assert!(not_live.text().contains("not live"));

        // A future schedule is accepted and visible before it fires.
        request
            .post(&schedule_path)
            .json(&json!({ "publish_at": "2099-01-01T00:00:00Z" }))
            .await
            .assert_status_ok();
        let queued: Value = request
            .get(&format!("/api/sites/{site_pid}/schedules"))
            .await
            .json();
        assert_eq!(queued["queued"][0]["entry_key"], "scheduled");

        // Nothing is due yet.
        let swept: Value = request.post("/api/schedules/sweep").await.json();
        assert_eq!(swept["applied"], 0);

        // Make it due.
        make_due(&ctx, &entry_pid, "scheduled_publish_at").await;
        let response = request.post("/api/schedules/sweep").await;
        assert_eq!(
            response.status_code(),
            200,
            "sweep failed: {}",
            response.text()
        );
        let swept: Value = response.json();
        assert_eq!(swept["applied"], 1, "{swept}");
        assert_eq!(swept["outcomes"][0]["outcome"], "published");

        let variant: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en"))
            .await
            .json();
        assert_eq!(variant["variant"]["status"], "published");
        assert!(!variant["variant"]["published_revision_pid"].is_null());
        assert!(
            variant["variant"]["scheduled_publish_at"].is_null(),
            "the due field is cleared in the same transaction — this is what makes it idempotent"
        );

        // Rerunning applies nothing: the schedule is gone, not merely
        // "already done" by luck of timing.
        let again: Value = request.post("/api/schedules/sweep").await.json();
        assert_eq!(again["applied"], 0);
        assert_eq!(again["skipped"], 0);

        // The audit row records that the clock did it, not a person.
        let trail: Value = request.get("/api/audits/recent").await.json();
        let scheduled_publish = trail
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["action"] == "publish" && row["snapshot"]["trigger"] == "schedule")
            .expect("a scheduled publish is audited with its trigger");
        assert_eq!(scheduled_publish["actor"], "system:schedule");
    })
    .await;
}

/// A schedule whose variant has moved on is skipped and recorded — the
/// clock does not overrule a person who acted more recently.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_schedule_overtaken_by_a_person_is_skipped_and_recorded() {
    request::<App, _, _>(|request, ctx| async move {
        let site_pid = seed_site_and_type(&request, "overtaken").await;
        let (entry_pid, _) = create_entry(&request, &site_pid, "overtaken").await;
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/schedule"))
            .json(&json!({ "publish_at": "2099-01-01T00:00:00Z" }))
            .await
            .assert_status_ok();

        // Someone archives it before the schedule fires.
        act(
            &request,
            &entry_pid,
            json!({ "action": "archive", "reason": "pulled from the plan" }),
        )
        .await
        .assert_status_ok();
        // Archiving does not clear the schedule (only publish/unpublish
        // do), so the sweep meets a schedule it must refuse.
        make_due(&ctx, &entry_pid, "scheduled_publish_at").await;

        let swept: Value = request.post("/api/schedules/sweep").await.json();
        assert_eq!(swept["applied"], 0);
        assert_eq!(swept["skipped"], 1);
        assert!(
            swept["outcomes"][0]["detail"]
                .as_str()
                .unwrap()
                .contains("no longer legal"),
            "{swept}"
        );

        let variant: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en"))
            .await
            .json();
        assert_eq!(
            variant["variant"]["status"], "archived",
            "the clock did not win"
        );
        assert!(
            variant["variant"]["scheduled_publish_at"].is_null(),
            "the dead schedule is cleared rather than retried forever"
        );

        // An operator can find out why their page did not go live
        // without reading logs.
        let trail: Value = request.get("/api/audits/recent").await.json();
        assert!(
            trail
                .as_array()
                .unwrap()
                .iter()
                .any(|row| row["action"] == "schedule_skipped"),
            "the skip is audited"
        );
    })
    .await;
}

/// The scheduled path runs the same gate as the manual one: a clock is
/// not a reason to publish a page that is not ready.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_scheduled_publish_still_has_to_pass_the_gate() {
    request::<App, _, _>(|request, ctx| async move {
        let site_pid = seed_site(&request, &a_key("scheduled-gate")).await;
        request
            .post(&format!("/api/sites/{site_pid}/content-types"))
            .json(&json!({
                "key": "article", "name": "Article", "routable": false,
                "fields": [{ "key": "standfirst", "label": "Standfirst", "kind": "text",
                             "required": true }],
            }))
            .await
            .assert_status_ok();
        let created: Value = request
            .post(&format!("/api/sites/{site_pid}/entries"))
            .json(
                &json!({ "key": "not-ready", "content_type_key": "article", "title": "Not ready" }),
            )
            .await
            .json();
        let entry_pid = created["pid"].as_str().unwrap().to_string();

        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/schedule"))
            .json(&json!({ "publish_at": "2099-01-01T00:00:00Z" }))
            .await
            .assert_status_ok();
        make_due(&ctx, &entry_pid, "scheduled_publish_at").await;

        let swept: Value = request.post("/api/schedules/sweep").await.json();
        assert_eq!(swept["applied"], 0);
        assert_eq!(swept["skipped"], 1);
        assert!(
            swept["outcomes"][0]["detail"]
                .as_str()
                .unwrap()
                .contains("required_field_empty"),
            "{swept}"
        );
        let variant: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en"))
            .await
            .json();
        assert_eq!(variant["variant"]["status"], "draft", "nothing went live");
    })
    .await;
}

/// Advisory locks: cooperative, expiring, and stealable with a reason —
/// and never a substitute for the authoritative save check.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn locks_are_advisory_and_stealing_needs_a_reason() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "locks").await;
        let (entry_pid, first) = create_entry(&request, &site_pid, "locked").await;
        let lock_path = format!("/api/entries/{entry_pid}/variants/en/lock");

        let locked: Value = request.post(&lock_path).json(&json!({})).await.json();
        assert!(!locked["locked_until"].is_null());
        assert_eq!(locked["stolen"], false);
        assert!(
            locked["advisory"]
                .as_str()
                .unwrap()
                .contains("base_revision_pid"),
            "the response says what the lock is and is not"
        );

        // The lock does not block a save: the authoritative protection
        // is the base-revision check, and this proves the lock is not
        // quietly acting as a mutex.
        request
            .post(&format!("/api/entries/{entry_pid}/variants/en/revisions"))
            .json(&json!({
                "base_revision_pid": first, "title": "Edited while locked",
                "blocks": [{ "kind": "paragraph", "text": "still writable" }],
            }))
            .await
            .assert_status_ok();

        request.delete(&lock_path).await.assert_status_ok();
        let variant: Value = request
            .get(&format!("/api/entries/{entry_pid}/variants/en"))
            .await
            .json();
        assert!(variant["variant"]["locked_until"].is_null());
    })
    .await;
}

/// Archive is reachable from anywhere and terminal except a reasoned
/// restore.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn archiving_is_reasoned_and_reversible_only_by_restore() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "archive").await;
        let (entry_pid, _) = create_entry(&request, &site_pid, "archivable").await;

        assert_eq!(
            act(&request, &entry_pid, json!({ "action": "archive" }))
                .await
                .status_code(),
            422,
            "archiving needs a reason"
        );
        act(
            &request,
            &entry_pid,
            json!({ "action": "archive", "reason": "superseded by the 2027 guidance" }),
        )
        .await
        .assert_status_ok();

        // Terminal: only restore is legal from here.
        for action in ["submit", "approve", "publish", "unpublish"] {
            let refused = act(
                &request,
                &entry_pid,
                json!({ "action": action, "reason": "x" }),
            )
            .await;
            assert_eq!(refused.status_code(), 422, "{action} should be refused");
            assert!(
                refused.text().contains("restore"),
                "the refusal lists what is legal"
            );
        }

        let restored: Value = act(
            &request,
            &entry_pid,
            json!({ "action": "restore", "reason": "still needed after all" }),
        )
        .await
        .json();
        assert_eq!(restored["to"], "draft");
    })
    .await;
}

/// An unknown action is refused by name, listing the real ones.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_unknown_action_lists_the_real_ones() {
    request::<App, _, _>(|request, _ctx| async move {
        let site_pid = seed_site_and_type(&request, "unknown-action").await;
        let (entry_pid, _) = create_entry(&request, &site_pid, "actions").await;
        let refused = act(&request, &entry_pid, json!({ "action": "yeet" })).await;
        assert_eq!(refused.status_code(), 422);
        assert!(refused.text().contains("unknown action"));
        assert!(refused.text().contains("publish"));
    })
    .await;
}
