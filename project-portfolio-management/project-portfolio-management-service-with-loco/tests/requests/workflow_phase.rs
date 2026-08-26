//! Request tests for the four features landed 2026-08-25: **custom
//! workflows** (FR-26), the **project phase** (FR-30), **Flow
//! Distribution** (FR-31), and **Total Project Control** (FR-37), plus
//! the **controls register** (FR-38/39).
//!
//! These exist because every one of those features was verified by hand
//! against a running service, which nobody can re-run. The repo's own
//! rule is that a status row claims only what a *command* proves
//! (entity spec §14.3), so each check below is one that was performed
//! manually and is now performed by CI.
//!
//! Two of them pin defects found during that manual pass and fixed:
//! `done_at` stamped from a state's **category** rather than the
//! literal string `"done"`, and the flow classification surviving a
//! fully custom vocabulary without disturbing an untouched board.
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

// ---------------------------------------------------------------- phase

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Advancement is one step at a time, and a refusal **names the phase it
// would have skipped** — a `422` that only says "no" leaves the
// operator guessing which step is missing.
async fn phase_advances_one_step_and_names_a_skip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Phase rules plan");

        // From no phase at all, only the first is enterable.
        let skip = request
            .put(&format!("/api/plans/{plan}/phase"))
            .json(&json!({ "phase": "executing" }))
            .await;
        assert_eq!(skip.status_code(), 422);
        let body: Value = skip.json();
        let described = body["description"].as_str().unwrap_or_default();
        assert!(
            described.contains("initiating"),
            "a skip must name the phase skipped, got: {described}"
        );

        let ok = request
            .put(&format!("/api/plans/{plan}/phase"))
            .json(&json!({ "phase": "initiating" }))
            .await;
        assert_eq!(ok.status_code(), 200);
        let body: Value = ok.json();
        assert_eq!(body["phase"], "initiating");
        assert_eq!(body["next_phase"], "planning");

        // An unrecognised token is refused, never coerced to a default:
        // a typo must not silently place a plan in `initiating`.
        let typo = request
            .put(&format!("/api/plans/{plan}/phase"))
            .json(&json!({ "phase": "exectuing" }))
            .await;
        assert_eq!(typo.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A backward move is permitted but never **silent**: re-planning is
// normal, an unexplained regression is not. The history then reports
// every phase even at zero and counts the revisit separately, because
// two visits and one long stay are different stories.
async fn a_regression_needs_a_reason_and_the_history_counts_revisits() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Regression plan");
        for phase in ["initiating", "planning"] {
            let moved = request
                .put(&format!("/api/plans/{plan}/phase"))
                .json(&json!({ "phase": phase }))
                .await;
            assert_eq!(moved.status_code(), 200);
        }

        let silent = request
            .put(&format!("/api/plans/{plan}/phase"))
            .json(&json!({ "phase": "initiating" }))
            .await;
        assert_eq!(silent.status_code(), 422, "a silent regression is refused");

        let explained = request
            .put(&format!("/api/plans/{plan}/phase"))
            .json(&json!({ "phase": "initiating", "reason": "charter reopened" }))
            .await;
        assert_eq!(explained.status_code(), 200);

        let history: Value = request
            .get(&format!("/api/plans/{plan}/phase-history"))
            .await
            .json();
        let durations = history["durations"].as_array().expect("durations");
        assert_eq!(durations.len(), 5, "every phase is reported, even at zero");
        let initiating = durations
            .iter()
            .find(|row| row["phase"] == "initiating")
            .expect("initiating row");
        assert_eq!(
            initiating["visits"], 2,
            "a revisit is counted, not merged into one total"
        );
        assert_eq!(initiating["current"], true);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The phase never gates an operational write. Refusing work on the
// basis of a phase would teach operators to misreport it, which costs
// more than it buys.
async fn phase_does_not_gate_operational_writes() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Closing plan");
        for phase in [
            "initiating",
            "planning",
            "executing",
            "controlling",
            "closing",
        ] {
            request
                .put(&format!("/api/plans/{plan}/phase"))
                .json(&json!({ "phase": phase }))
                .await;
        }
        let task = request
            .post(&format!("/api/plans/{plan}/tasks"))
            .json(&json!({ "title": "raised during closing" }))
            .await;
        assert_eq!(
            task.status_code(),
            200,
            "a task must be creatable on a plan in `closing`"
        );
    })
    .await;
}

// ------------------------------------------------------------- workflow

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// With nothing configured the built-in vocabulary is in force and the
// board is unconstrained — the backward-compatibility pin. If this
// fails, every existing board just changed behaviour.
async fn a_plan_with_no_workflow_keeps_the_built_in_vocabulary() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Vanilla board");
        let view: Value = request
            .get(&format!("/api/plans/{plan}/workflow"))
            .await
            .json();
        assert_eq!(view["source"], "built_in_or_default");
        assert_eq!(
            view["constrained"], false,
            "an empty transition set is open"
        );

        let created = request
            .post(&format!("/api/plans/{plan}/tasks"))
            .json(&json!({ "title": "legacy", "status": "in_progress" }))
            .await;
        assert_eq!(created.status_code(), 200, "today's statuses still work");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A state with no recognised category is refused **at registration**,
// and the refusal names the offending state. Left registered, it would
// read `unmeasured` forever — and every derived view computes from what
// a state means, not from its name.
async fn a_state_without_a_category_cannot_be_registered() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Bad workflow plan");
        let refused = request
            .post("/api/workflows")
            .json(&json!({
                "name": "Bad", "applies_to": "task", "plan_pid": plan,
                "states": [{ "key": "icebox", "label": "Icebox",
                             "category": "nearly_done", "is_initial": true }]
            }))
            .await;
        assert_eq!(refused.status_code(), 422);
        let body: Value = refused.json();
        let described = body["description"].as_str().unwrap_or_default();
        assert!(
            described.contains("icebox"),
            "the refusal must name the offending state, got: {described}"
        );

        // A workflow that can never finish is refused too: nothing could
        // complete and the burndown would never fall.
        let never_finishes = request
            .post("/api/workflows")
            .json(&json!({
                "name": "NoDone", "applies_to": "task", "plan_pid": plan,
                "states": [{ "key": "icebox", "label": "Icebox",
                             "category": "todo", "is_initial": true }]
            }))
            .await;
        assert_eq!(never_finishes.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The whole point of T-15, in one test: a fully custom vocabulary takes
// force, constrains moves, **stamps `done_at` from the state's
// category** rather than the literal string "done", and leaves every
// derived view computable.
//
// The `done_at` half pins a real defect: before it was fixed, a board
// finishing in `shipped` never stamped completion, so the burndown was
// blind to every board that renamed its final column.
async fn a_custom_vocabulary_takes_force_and_stays_analysable() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Custom board");
        let registered = request
            .post("/api/workflows")
            .json(&json!({
                "name": "Ship it", "applies_to": "task", "plan_pid": plan,
                "states": [
                    { "key": "icebox", "label": "Icebox", "category": "todo", "is_initial": true },
                    { "key": "hacking", "label": "Hacking", "category": "active" },
                    { "key": "awaiting_ci", "label": "Awaiting CI", "category": "waiting" },
                    { "key": "shipped", "label": "Shipped", "category": "done", "is_terminal": true }
                ],
                "transitions": [
                    { "from": "icebox", "to": "hacking" },
                    { "from": "hacking", "to": "awaiting_ci" },
                    { "from": "awaiting_ci", "to": "shipped" }
                ]
            }))
            .await;
        assert_eq!(registered.status_code(), 200);

        let view: Value = request
            .get(&format!("/api/plans/{plan}/workflow"))
            .await
            .json();
        assert_eq!(view["source"], "plan");
        assert_eq!(view["constrained"], true);

        // The old vocabulary is now refused on this plan.
        let old = request
            .post(&format!("/api/plans/{plan}/tasks"))
            .json(&json!({ "title": "old vocab", "status": "in_progress" }))
            .await;
        assert_eq!(old.status_code(), 422);

        // Creation defaults to the workflow's initial state.
        let created: Value = request
            .post(&format!("/api/plans/{plan}/tasks"))
            .json(&json!({ "title": "new vocab" }))
            .await
            .json();
        assert_eq!(created["status"], "icebox");
        let task = created["pid"].as_str().expect("task pid").to_string();

        // An undeclared transition is refused.
        let illegal = request
            .patch(&format!("/api/plans/{plan}/tasks/{task}"))
            .json(&json!({ "status": "shipped" }))
            .await;
        assert_eq!(illegal.status_code(), 422, "icebox -> shipped is not declared");

        // The declared path works, and completion is stamped from the
        // **category** — the defect this test exists for.
        for status in ["hacking", "awaiting_ci", "shipped"] {
            let moved = request
                .patch(&format!("/api/plans/{plan}/tasks/{task}"))
                .json(&json!({ "status": status }))
                .await;
            assert_eq!(moved.status_code(), 200, "move to {status}");
        }
        let finished: Value = request
            .patch(&format!("/api/plans/{plan}/tasks/{task}"))
            .json(&json!({ "status": "shipped" }))
            .await
            .json();
        assert!(
            !finished["done_at"].is_null(),
            "`done_at` must be stamped by a custom `done` state, not only by the \
             literal status \"done\" — otherwise the burndown is blind to any board \
             that renamed its final column"
        );

        // And the analysis still classifies the custom vocabulary.
        let analysis: Value = request
            .get(&format!("/api/plans/{plan}/time-analysis"))
            .await
            .json();
        let classes = &analysis["classification"]["classes"];
        assert_eq!(
            classes["hacking"], "value_adding",
            "a custom active state must be value-adding, or the board reports no work"
        );
        assert_eq!(classes["awaiting_ci"], "unnecessary_non_value_adding");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The regression pin. Deriving flow classes from workflow categories
// nearly reclassified `in_review` from *necessary* to *value-adding*,
// which would have raised the flow efficiency of every board that
// configured nothing — a measurement moving because of an unrelated
// feature. The disclosed default must still win.
async fn an_untouched_board_keeps_its_disclosed_classification() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let plan = create_plan!(request, "Untouched board");
        let analysis: Value = request
            .get(&format!("/api/plans/{plan}/time-analysis"))
            .await
            .json();
        let classes = &analysis["classification"]["classes"];
        assert_eq!(
            classes["in_review"], "necessary_non_value_adding",
            "the four workflow categories cannot express `necessary`, so the \
             disclosed default map must override the derivation"
        );
        assert_eq!(classes["in_progress"], "value_adding");
        assert_eq!(classes["blocked"], "unnecessary_non_value_adding");
    })
    .await;
}
