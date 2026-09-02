//! Request tests for the collaboration / automation / prioritisation
//! capabilities: collaborative review end-to-end, assignee management,
//! workflow automation firing on a board move, the set-and-forget
//! sweep, the Smart Score breakdown, and bird's-eye lifecycle
//! visibility.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

/// Create a plan through the API and return its pid.
macro_rules! create_plan {
    ($request:expr, $name:expr) => {{
        let created: Value = $request
            .post("/api/plans")
            .json(&json!({ "name": $name }))
            .await
            .json();
        created["pid"]
            .as_str()
            .or_else(|| created["plan"]["pid"].as_str())
            .expect("plan pid")
            .to_string()
    }};
}

/// A person EntityRef URN.
fn person(uuid: &str) -> String {
    format!("person:{uuid}")
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Collaborative review end-to-end: delegate to two experts (one
// external), one declines, one accepts and submits — and the consensus
// reports the outstanding invitation rather than declaring agreement.
async fn collaborative_review_delegates_and_aggregates_honestly() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Reviewable plan");
        let expert_a = person("11111111-1111-1111-1111-111111111111");
        let expert_b = person("22222222-2222-2222-2222-222222222222");
        let expert_c = person("33333333-3333-3333-3333-333333333333");

        let invite = |reviewer: String, scope: &str| {
            let scope = scope.to_string();
            let plan_pid = plan_pid.clone();
            let request = &request;
            async move {
                request
                    .post("/api/reviews")
                    .json(&json!({
                        "subject_kind": "plan",
                        "subject_pid": plan_pid,
                        "reviewer_ref": reviewer,
                        "reviewer_scope": scope,
                        "expertise": "clinical safety",
                    }))
                    .await
            }
        };
        let first: Value = invite(expert_a.clone(), "internal").await.json();
        let second: Value = invite(expert_b.clone(), "external").await.json();
        let third: Value = invite(expert_c.clone(), "internal").await.json();
        assert_eq!(first["status"], "invited");
        assert_eq!(second["reviewer_scope"], "external");

        // A duplicate live invitation for the same expert is refused.
        assert_eq!(
            invite(expert_a.clone(), "internal").await.status_code(),
            422,
            "the same expert must not be invited twice at once"
        );

        // An unaccepted invitation cannot submit a verdict.
        let a_pid = first["pid"].as_str().expect("pid");
        assert_eq!(
            request
                .post(&format!("/api/reviews/{a_pid}/submit"))
                .json(&json!({ "recommendation": "advance" }))
                .await
                .status_code(),
            422,
            "an unanswered invitation must not become evidence"
        );

        // A accepts and submits; B declines; C is left outstanding.
        request
            .post(&format!("/api/reviews/{a_pid}/respond"))
            .json(&json!({ "response": "accept" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/reviews/{a_pid}/submit"))
            .json(&json!({ "score": 80, "recommendation": "advance", "comment": "Solid" }))
            .await
            .assert_status_ok();
        let b_pid = second["pid"].as_str().expect("pid");
        request
            .post(&format!("/api/reviews/{b_pid}/respond"))
            .json(&json!({ "response": "decline" }))
            .await
            .assert_status_ok();
        let _ = third;

        let consensus: Value = request
            .get(&format!(
                "/api/reviews/consensus?subject_kind=plan&subject_pid={plan_pid}"
            ))
            .await
            .json();
        assert_eq!(consensus["submitted"], 1);
        assert_eq!(consensus["declined"], 1);
        assert_eq!(consensus["outstanding"], 1);
        assert_eq!(
            consensus["complete"], false,
            "one expert still owes a verdict"
        );
        assert_eq!(consensus["mean_score"], 80.0);
        assert_eq!(
            consensus["majority"], "advance",
            "1 of 1 submitted verdicts is a strict majority"
        );

        // The reviewer was told they had been asked.
        let inbox: Value = request
            .get(&format!("/api/notifications?recipient={expert_a}"))
            .await
            .json();
        assert!(
            inbox.as_array().is_some_and(|rows| !rows.is_empty()),
            "the invited expert has an in-app notification"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins pagination (agents/share/restful.md) on the two collaboration
// list endpoints — the delegation list (`GET /api/reviews`) and one
// inbox (`GET /api/notifications`): `limit`/`offset` window the rows,
// `X-Total-Count` reports the whole match set, the limit clamps, and an
// out-of-bound offset is a `400`. Mirrors `/api/plans`'s pagination
// test (`tests/requests/plans.rs`).
async fn collaboration_lists_are_paginated() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let reviewer = person("77777777-7777-7777-7777-777777777777");

        macro_rules! header {
            ($r:expr, $name:expr) => {
                $r.headers()
                    .get($name)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string()
            };
        }

        // Five plans, each delegated to the same reviewer: five live
        // `reviews` rows, and five `review_invited` notifications land
        // in that reviewer's inbox as a side effect.
        for i in 0..5 {
            let plan_pid = create_plan!(request, format!("Delegation roster plan {i}"));
            request
                .post("/api/reviews")
                .json(&json!({
                    "subject_kind": "plan",
                    "subject_pid": plan_pid,
                    "reviewer_ref": reviewer,
                }))
                .await
                .assert_status_ok();
        }

        let page = request.get("/api/reviews?limit=2&offset=1").await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.json::<Value>().as_array().expect("array").len(), 2);
        assert_eq!(header!(page, "x-total-count"), "5");
        assert_eq!(header!(page, "x-limit"), "2");
        assert_eq!(header!(page, "x-offset"), "1");

        let all = request.get("/api/reviews").await;
        assert_eq!(all.json::<Value>().as_array().expect("array").len(), 5);
        assert_eq!(header!(all, "x-limit"), "200", "the default is the old cap");

        let clamped = request.get("/api/reviews?limit=100000").await;
        assert_eq!(header!(clamped, "x-limit"), "500");

        assert_eq!(
            request.get("/api/reviews?offset=10001").await.status_code(),
            400
        );

        let inbox = request
            .get(&format!(
                "/api/notifications?recipient={reviewer}&limit=2&offset=1"
            ))
            .await;
        assert_eq!(inbox.status_code(), 200);
        assert_eq!(inbox.json::<Value>().as_array().expect("array").len(), 2);
        assert_eq!(header!(inbox, "x-total-count"), "5");
        assert_eq!(header!(inbox, "x-limit"), "2");
        assert_eq!(header!(inbox, "x-offset"), "1");

        assert_eq!(
            request
                .get(&format!(
                    "/api/notifications?recipient={reviewer}&offset=10001"
                ))
                .await
                .status_code(),
            400
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A rule configured once fires when the task crosses the board, logs
// what it did, and a rule scoped to another plan stays out of it.
async fn workflow_automation_fires_on_a_board_move_and_logs_the_run() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Automated board");
        let other_pid = create_plan!(request, "Someone else's board");
        let assignee = person("44444444-4444-4444-4444-444444444444");

        // Rule 1: anything entering `in_review` is assigned to the lead.
        let rule: Value = request
            .post("/api/automations")
            .json(&json!({
                "plan_pid": plan_pid,
                "name": "Route reviews to the lead",
                "trigger_kind": "task_moved",
                "to_status": "in_review",
                "actions": [
                    { "kind": "assign", "value": { "assignee_ref": assignee } },
                ],
            }))
            .await
            .json();
        assert_eq!(rule["enabled"], true);

        // Rule 2: the same trigger on a different plan must not fire.
        request
            .post("/api/automations")
            .json(&json!({
                "plan_pid": other_pid,
                "name": "Other board only",
                "trigger_kind": "task_moved",
                "to_status": "in_review",
                "actions": [
                    { "kind": "add_label", "value": { "label": "should-not-appear" } },
                ],
            }))
            .await
            .assert_status_ok();

        // A malformed action is refused at write time.
        assert_eq!(
            request
                .post("/api/automations")
                .json(&json!({
                    "name": "Bad rule",
                    "trigger_kind": "task_moved",
                    "actions": [
                        { "kind": "assign", "value": { "assignee_ref": "just-a-name" } },
                    ],
                }))
                .await
                .status_code(),
            422
        );

        // An empty actions array is refused too — it could never do
        // anything.
        assert_eq!(
            request
                .post("/api/automations")
                .json(&json!({
                    "name": "No actions",
                    "trigger_kind": "task_moved",
                    "actions": [],
                }))
                .await
                .status_code(),
            422
        );

        let task: Value = request
            .post(&format!("/api/plans/{plan_pid}/tasks"))
            .json(&json!({ "title": "Draft the thing" }))
            .await
            .json();
        let task_pid = task["pid"].as_str().expect("task pid").to_string();

        // Moving into a column with no rule changes nothing automatic.
        request
            .patch(&format!("/api/plans/{plan_pid}/tasks/{task_pid}"))
            .json(&json!({ "status": "in_progress" }))
            .await
            .assert_status_ok();
        let quiet: Value = request.get("/api/automations/runs").await.json();
        assert_eq!(
            quiet.as_array().map(Vec::len),
            Some(0),
            "no rule matched that move"
        );

        // Moving into `in_review` fires the rule.
        request
            .patch(&format!("/api/plans/{plan_pid}/tasks/{task_pid}"))
            .json(&json!({ "status": "in_review" }))
            .await
            .assert_status_ok();
        let moved: Value = request
            .get(&format!("/api/plans/{plan_pid}/tasks"))
            .await
            .json();
        // `GET /tasks` answers `{ "tasks": [...], "counts": {...} }`, not a
        // bare array — indexing it with `[0]` yielded `Null`, so this
        // assertion failed while the automation had in fact run: the rule
        // logged an `applied` run and the row really did carry the
        // assignee.
        assert_eq!(
            moved["tasks"][0]["assignee_ref"], assignee,
            "the automation assigned the task"
        );

        let runs: Value = request.get("/api/automations/runs").await.json();
        let rows = runs.as_array().expect("runs");
        assert_eq!(rows.len(), 1, "exactly one rule matched: {rows:?}");
        assert_eq!(rows[0]["outcome"], "applied");
        assert_eq!(rows[0]["automation_pid"], rule["pid"]);

        // The other plan's rule never touched this plan's payload.
        let plan: Value = request.get(&format!("/api/plans/{plan_pid}")).await.json();
        let tags = plan["tags"].as_array().cloned().unwrap_or_default();
        assert!(
            !tags.iter().any(|t| t == "should-not-appear"),
            "a plan-scoped rule must not fire on another plan: {tags:?}"
        );

        // Disabling stops it firing again.
        let rule_pid = rule["pid"].as_str().expect("pid");
        request
            .post(&format!("/api/automations/{rule_pid}/disable"))
            .await
            .assert_status_ok();
        request
            .patch(&format!("/api/plans/{plan_pid}/tasks/{task_pid}"))
            .json(&json!({ "status": "todo" }))
            .await
            .assert_status_ok();
        request
            .patch(&format!("/api/plans/{plan_pid}/tasks/{task_pid}"))
            .json(&json!({ "status": "in_review" }))
            .await
            .assert_status_ok();
        let after: Value = request.get("/api/automations/runs").await.json();
        assert_eq!(
            after.as_array().map(Vec::len),
            Some(1),
            "a disabled rule adds no runs"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// FR-32: a rule with more than one action applies every one of them, in
// the array's declared order, and logs each action's own outcome
// separately — one action skipping (the plan already carries the
// label) does not stop the next one from applying.
async fn a_multi_action_rule_applies_every_action_in_order_and_logs_each_outcome() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Multi-action board");
        let assignee = person("66666666-6666-6666-6666-666666666666");

        // The plan already carries this label, so the add_label action
        // (declared first) will skip — the assign action (declared
        // second) must still apply.
        request
            .put(&format!("/api/plans/{plan_pid}"))
            .json(&json!({ "name": "Multi-action board", "tags": ["already-tagged"] }))
            .await
            .assert_status_ok();

        let rule: Value = request
            .post("/api/automations")
            .json(&json!({
                "plan_pid": plan_pid,
                "name": "Label then assign",
                "trigger_kind": "task_moved",
                "to_status": "in_review",
                "actions": [
                    { "kind": "add_label", "value": { "label": "already-tagged" } },
                    { "kind": "assign", "value": { "assignee_ref": assignee } },
                ],
            }))
            .await
            .json();
        let rule_pid = rule["pid"].as_str().expect("pid").to_string();

        let task: Value = request
            .post(&format!("/api/plans/{plan_pid}/tasks"))
            .json(&json!({ "title": "Needs both actions" }))
            .await
            .json();
        let task_pid = task["pid"].as_str().expect("task pid").to_string();

        request
            .patch(&format!("/api/plans/{plan_pid}/tasks/{task_pid}"))
            .json(&json!({ "status": "in_review" }))
            .await
            .assert_status_ok();

        // The second action applied despite the first one skipping.
        let moved: Value = request
            .get(&format!("/api/plans/{plan_pid}/tasks"))
            .await
            .json();
        assert_eq!(
            moved["tasks"][0]["assignee_ref"], assignee,
            "the second action (assign) must still apply after the first skipped"
        );

        // Both actions logged their own outcome, in declared order.
        let runs: Value = request
            .get(&format!("/api/automations/runs?automation={rule_pid}"))
            .await
            .json();
        let mut rows = runs.as_array().cloned().expect("runs");
        assert_eq!(rows.len(), 2, "one row per action: {rows:?}");
        rows.sort_by_key(|r| r["action_index"].as_i64().unwrap_or(-1));
        assert_eq!(rows[0]["action_index"], 0);
        assert_eq!(rows[0]["outcome"], "skipped", "already carried the label");
        assert_eq!(rows[1]["action_index"], 1);
        assert_eq!(rows[1]["outcome"], "applied", "the assign action");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// FR-32: "a date arriving" — a `milestone_due` rule fires once a
// milestone's due date has passed, and the sweep is exactly-once: a
// second sweep over the same still-overdue milestone does not refire
// the rule.
async fn a_milestone_due_rule_fires_once_the_date_arrives_and_never_again() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Milestone-triggered board");
        let recipient = person("77777777-7777-7777-7777-777777777777");

        let milestone: Value = request
            .post(&format!("/api/plans/{plan_pid}/milestones"))
            .json(&json!({ "name": "Beta freeze", "due": "2020-01-01" }))
            .await
            .json();
        let milestone_pid = milestone["pid"]
            .as_str()
            .expect("milestone pid")
            .to_string();

        // A milestone due in the far future must not fire.
        let future_milestone: Value = request
            .post(&format!("/api/plans/{plan_pid}/milestones"))
            .json(&json!({ "name": "Not yet", "due": "2099-01-01" }))
            .await
            .json();
        let future_pid = future_milestone["pid"]
            .as_str()
            .expect("future milestone pid")
            .to_string();

        let rule: Value = request
            .post("/api/automations")
            .json(&json!({
                "plan_pid": plan_pid,
                "name": "Notify on milestone due",
                "trigger_kind": "milestone_due",
                "actions": [
                    { "kind": "notify", "value": { "recipient_ref": recipient } },
                ],
            }))
            .await
            .json();
        let rule_pid = rule["pid"].as_str().expect("pid").to_string();

        let first: Value = request
            .post("/api/automations/milestones/sweep")
            .await
            .json();
        assert_eq!(
            first["fired"], 1,
            "exactly the one overdue milestone: {first:?}"
        );
        assert_eq!(first["already_claimed"], 0);

        let runs: Value = request
            .get(&format!("/api/automations/runs?automation={rule_pid}"))
            .await
            .json();
        let rows = runs.as_array().cloned().expect("runs");
        assert_eq!(rows.len(), 1, "one run for the one overdue milestone");
        assert_eq!(rows[0]["subject_pid"], milestone_pid);
        assert_eq!(rows[0]["outcome"], "applied");

        // A second sweep claims nothing new: the pair already fired.
        let second: Value = request
            .post("/api/automations/milestones/sweep")
            .await
            .json();
        assert_eq!(second["fired"], 0, "already fired for this milestone");
        assert_eq!(second["already_claimed"], 1);
        let after: Value = request
            .get(&format!("/api/automations/runs?automation={rule_pid}"))
            .await
            .json();
        assert_eq!(
            after.as_array().map(Vec::len),
            Some(1),
            "still exactly one run, not two"
        );

        // The far-future milestone never appears in any run.
        let none = rows.iter().any(|r| r["subject_pid"] == future_pid);
        assert!(!none, "a not-yet-due milestone must not fire");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Set and forget: a deadline in the future is not fired by a sweep, and
// a cancelled one never fires at all.
async fn scheduled_actions_only_fire_when_due_and_only_once() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Deadline plan");
        let recipient = person("55555555-5555-5555-5555-555555555555");

        let future: Value = request
            .post("/api/scheduled-actions")
            .json(&json!({
                "subject_kind": "plan",
                "subject_pid": plan_pid,
                "action_kind": "notify",
                "in_days": 30,
                "recipient_ref": recipient,
                "message": "Gate review due",
            }))
            .await
            .json();
        assert_eq!(future["status"], "pending");

        // A notify with nowhere to send it is refused.
        assert_eq!(
            request
                .post("/api/scheduled-actions")
                .json(&json!({
                    "subject_kind": "plan",
                    "subject_pid": plan_pid,
                    "action_kind": "notify",
                    "in_days": 5,
                }))
                .await
                .status_code(),
            422
        );
        // So is a horizon beyond the cap.
        assert_eq!(
            request
                .post("/api/scheduled-actions")
                .json(&json!({
                    "subject_kind": "plan",
                    "subject_pid": plan_pid,
                    "action_kind": "expire_review",
                    "in_days": 4000,
                }))
                .await
                .status_code(),
            422
        );

        // Nothing is due yet.
        let swept: Value = request.post("/api/scheduled-actions/sweep").await.json();
        assert_eq!(swept["fired"], 0, "a future deadline must not fire early");
        let queue: Value = request
            .get("/api/scheduled-actions?status=pending")
            .await
            .json();
        assert_eq!(queue.as_array().map(Vec::len), Some(1));

        // Cancelling removes it from the queue; cancelling twice refuses.
        let pid = future["pid"].as_str().expect("pid");
        request
            .delete(&format!("/api/scheduled-actions/{pid}"))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .delete(&format!("/api/scheduled-actions/{pid}"))
                .await
                .status_code(),
            404,
            "a cancelled action is gone"
        );
        let after: Value = request.post("/api/scheduled-actions/sweep").await.json();
        assert_eq!(after["fired"], 0, "a cancelled deadline never fires");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins pagination (agents/share/restful.md) on the three automation
// list endpoints — configured rules, their run log, and the deadline
// queue: `limit`/`offset` window the rows, `X-Total-Count` reports the
// whole match set, the limit clamps, and an out-of-bound offset is a
// `400`. The deadline queue's soonest-first order must hold under
// paging — a page must be a contiguous slice of the sorted order, not
// a reshuffled one. Mirrors `/api/plans`'s pagination test
// (`tests/requests/plans.rs`).
async fn automation_lists_are_paginated() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Automation ruleset board");

        macro_rules! header {
            ($r:expr, $name:expr) => {
                $r.headers()
                    .get($name)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string()
            };
        }

        // Five automations, all scoped to this plan and all matching the
        // same trigger, so one board move fires every one of them.
        for i in 0..5 {
            request
                .post("/api/automations")
                .json(&json!({
                    "plan_pid": plan_pid,
                    "name": format!("Rule {i}"),
                    "trigger_kind": "task_moved",
                    "to_status": "in_review",
                    "actions": [
                        { "kind": "add_label", "value": { "label": format!("paging-label-{i}") } },
                    ],
                }))
                .await
                .assert_status_ok();
        }

        let page = request.get("/api/automations?limit=2&offset=1").await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.json::<Value>().as_array().expect("array").len(), 2);
        assert_eq!(header!(page, "x-total-count"), "5");
        assert_eq!(header!(page, "x-limit"), "2");
        assert_eq!(header!(page, "x-offset"), "1");

        let all = request.get("/api/automations").await;
        assert_eq!(all.json::<Value>().as_array().expect("array").len(), 5);
        assert_eq!(header!(all, "x-limit"), "200", "the default is the old cap");

        let clamped = request.get("/api/automations?limit=100000").await;
        assert_eq!(header!(clamped, "x-limit"), "500");

        assert_eq!(
            request
                .get("/api/automations?offset=10001")
                .await
                .status_code(),
            400
        );

        // Firing every rule at once populates the run log.
        let task: Value = request
            .post(&format!("/api/plans/{plan_pid}/tasks"))
            .json(&json!({ "title": "Trigger the rules" }))
            .await
            .json();
        let task_pid = task["pid"].as_str().expect("task pid").to_string();
        request
            .patch(&format!("/api/plans/{plan_pid}/tasks/{task_pid}"))
            .json(&json!({ "status": "in_review" }))
            .await
            .assert_status_ok();

        let runs_page = request.get("/api/automations/runs?limit=2&offset=1").await;
        assert_eq!(runs_page.status_code(), 200);
        assert_eq!(
            runs_page.json::<Value>().as_array().expect("array").len(),
            2
        );
        assert_eq!(header!(runs_page, "x-total-count"), "5");
        assert_eq!(header!(runs_page, "x-limit"), "2");
        assert_eq!(header!(runs_page, "x-offset"), "1");

        // Five deadlines at different horizons; the queue must page
        // through them soonest-first.
        for days in [10, 5, 20, 1, 15] {
            request
                .post("/api/scheduled-actions")
                .json(&json!({
                    "subject_kind": "plan",
                    "subject_pid": plan_pid,
                    "action_kind": "expire_review",
                    "in_days": days,
                }))
                .await
                .assert_status_ok();
        }

        let full: Value = request.get("/api/scheduled-actions?limit=10").await.json();
        let full_rows = full.as_array().expect("rows");
        assert_eq!(full_rows.len(), 5);
        let due_ats: Vec<&str> = full_rows
            .iter()
            .map(|r| r["due_at"].as_str().expect("due_at"))
            .collect();
        let mut sorted = due_ats.clone();
        sorted.sort_unstable();
        assert_eq!(due_ats, sorted, "the unpaginated queue is soonest-first");

        let queue_page = request.get("/api/scheduled-actions?limit=2&offset=2").await;
        assert_eq!(queue_page.status_code(), 200);
        let page_rows: Value = queue_page.json();
        let page_arr = page_rows.as_array().expect("array");
        assert_eq!(page_arr.len(), 2);
        assert_eq!(header!(queue_page, "x-total-count"), "5");
        assert_eq!(
            page_arr[0]["due_at"], full_rows[2]["due_at"],
            "page 2 (offset=2) starts where the unpaginated order says it should"
        );
        assert_eq!(page_arr[1]["due_at"], full_rows[3]["due_at"]);

        assert_eq!(
            request
                .get("/api/scheduled-actions?offset=10001")
                .await
                .status_code(),
            400
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Assignee management: assign, see the workload (including unassigned),
// then unassign.
async fn assignee_workload_shows_the_open_pile_including_unassigned() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Staffed plan");
        let worker = person("66666666-6666-6666-6666-666666666666");
        let mut pids = Vec::new();
        for title in ["One", "Two"] {
            let task: Value = request
                .post(&format!("/api/plans/{plan_pid}/tasks"))
                .json(&json!({ "title": title }))
                .await
                .json();
            pids.push(task["pid"].as_str().expect("pid").to_string());
        }
        request
            .post(&format!("/api/plans/{plan_pid}/tasks/{}/assign", pids[0]))
            .json(&json!({ "assignee_ref": worker }))
            .await
            .assert_status_ok();

        let load: Value = request.get("/api/assignees/workload").await.json();
        let rows = load["assignees"].as_array().expect("assignees");
        assert_eq!(rows.len(), 2, "the assignee and the unassigned pile");
        assert!(
            rows.iter().any(|r| r["assignee_ref"] == "unassigned"),
            "unassigned work is surfaced: {rows:?}"
        );

        // A bad reference is refused; null unassigns.
        assert_eq!(
            request
                .post(&format!("/api/plans/{plan_pid}/tasks/{}/assign", pids[0]))
                .json(&json!({ "assignee_ref": "jo" }))
                .await
                .status_code(),
            422
        );
        let unassigned: Value = request
            .post(&format!("/api/plans/{plan_pid}/tasks/{}/assign", pids[0]))
            .json(&json!({ "assignee_ref": null }))
            .await
            .json();
        assert!(unassigned["assignee_ref"].is_null());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The Smart Score explains itself: a plan with no evidence scores null
// (not zero), and evidence moves the score and shrinks `missing`.
async fn smart_score_explains_itself_and_never_fakes_evidence() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Scored plan");
        let scored: Value = request
            .get(&format!("/api/plans/{plan_pid}/smart-score"))
            .await
            .json();
        let score = &scored["smart_score"];
        // A brand-new plan has exactly one piece of evidence: momentum.
        assert_eq!(score["band"], "high", "just-updated plans have momentum");
        assert!(
            score["missing"]
                .as_array()
                .expect("missing")
                .iter()
                .any(|m| m == "roi"),
            "the absent evidence is named: {score:?}"
        );
        assert!(
            score["coverage"].as_f64().expect("coverage") < 1.0,
            "thin evidence is disclosed"
        );

        // Add an expert verdict; the review component appears.
        let expert = person("77777777-7777-7777-7777-777777777777");
        let review: Value = request
            .post("/api/reviews")
            .json(&json!({
                "subject_kind": "plan", "subject_pid": plan_pid, "reviewer_ref": expert,
            }))
            .await
            .json();
        let review_pid = review["pid"].as_str().expect("pid");
        request
            .post(&format!("/api/reviews/{review_pid}/respond"))
            .json(&json!({ "response": "accept" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/reviews/{review_pid}/submit"))
            .json(&json!({ "score": 90, "recommendation": "advance" }))
            .await
            .assert_status_ok();

        let rescored: Value = request
            .get(&format!("/api/plans/{plan_pid}/smart-score"))
            .await
            .json();
        let components = rescored["smart_score"]["components"]
            .as_array()
            .expect("components");
        assert!(
            components.iter().any(|c| c["name"] == "expert_review"),
            "the verdict became evidence: {components:?}"
        );
        assert!(
            rescored["smart_score"]["coverage"]
                .as_f64()
                .expect("coverage")
                > scored["smart_score"]["coverage"]
                    .as_f64()
                    .expect("coverage"),
            "coverage grew with the evidence"
        );

        // The ranked queue includes it and carries the band.
        let ranked: Value = request.get("/api/prioritisation").await.json();
        let plans = ranked["plans"].as_array().expect("plans");
        assert!(plans.iter().any(|p| p["pid"] == plan_pid.as_str()));
        assert!(plans[0]["band"].is_string());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Bird's-eye visibility: every phase is reported, and a plan's
// readiness names each blocker instead of just saying "not ready".
async fn lifecycle_reports_every_phase_and_names_each_blocker() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan_pid = create_plan!(request, "Lifecycle plan");
        let funnel: Value = request.get("/api/lifecycle").await.json();
        let phases = funnel["phases"].as_array().expect("phases");
        assert_eq!(phases.len(), 6, "every phase appears, even at zero");
        assert!(
            phases
                .iter()
                .any(|p| p["phase"] == "in_delivery" && p["live"] == 1),
            "the new plan is in delivery: {phases:?}"
        );

        // A clean plan is ready for its first gate.
        let clean: Value = request
            .get(&format!("/api/plans/{plan_pid}/lifecycle"))
            .await
            .json();
        assert_eq!(clean["readiness"]["ready"], true);
        assert_eq!(clean["readiness"]["next_gate"], "g0_concept");

        // Block it: an outstanding review and a blocked task.
        let expert = person("88888888-8888-8888-8888-888888888888");
        request
            .post("/api/reviews")
            .json(&json!({
                "subject_kind": "plan", "subject_pid": plan_pid, "reviewer_ref": expert,
            }))
            .await
            .assert_status_ok();
        let task: Value = request
            .post(&format!("/api/plans/{plan_pid}/tasks"))
            .json(&json!({ "title": "Stuck" }))
            .await
            .json();
        let task_pid = task["pid"].as_str().expect("pid");
        request
            .patch(&format!("/api/plans/{plan_pid}/tasks/{task_pid}"))
            .json(&json!({ "status": "blocked" }))
            .await
            .assert_status_ok();

        let blocked: Value = request
            .get(&format!("/api/plans/{plan_pid}/lifecycle"))
            .await
            .json();
        assert_eq!(blocked["readiness"]["ready"], false);
        let blockers = blocked["readiness"]["blockers"]
            .as_array()
            .expect("blockers");
        assert!(
            blockers.iter().any(|b| b == "reviews_answered"),
            "{blockers:?}"
        );
        assert!(
            blockers.iter().any(|b| b == "nothing_blocked"),
            "{blockers:?}"
        );
        assert_eq!(
            blocked["readiness"]["checks"].as_array().map(Vec::len),
            Some(5),
            "every check is reported, passing or not"
        );
    })
    .await;
}
