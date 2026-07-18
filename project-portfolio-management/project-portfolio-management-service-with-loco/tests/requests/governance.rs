//! PPM Phase-A governance request tests (spec/15-roadmap PPM-1/3/10/12):
//! the intake pipeline end-to-end (draft → … → promote mints the work
//! item), duplicate-demand detection, the strictly-ordered gate
//! journey, risks, and budget arithmetic — over the live routes.
//!
//! `#[ignore]`d: needs PostgreSQL (`config/test.yaml` /
//! `DATABASE_URL`); run with `cargo test -- --ignored`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

/// A draft proposal payload targeting the `projects` collection.
fn a_proposal(title: &str) -> Value {
    json!({
        "title": title,
        "summary": "Replace the legacy platform",
        "kind_target": "projects",
        "sponsor_ref": format!("organization:{}", uuid::Uuid::new_v4()),
        "requested_minor": 25_000_000_i64,
        "currency": "GBP",
    })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The full intake pipeline: draft → submitted → in_review → approved →
// promoted, which mints a real work item in the target collection;
// out-of-order actions and post-submission edits are refused.
async fn intake_pipeline_promotes_into_the_registry() {
    request::<App, _, _>(|request, _ctx| async move {
        // Bad target collection is 422; a valid draft creates.
        let bad = request
            .post("/api/proposals")
            .json(&json!({ "title": "X", "kind_target": "initiatives" }))
            .await;
        assert_eq!(bad.status_code(), 422);
        let created: Value = request
            .post("/api/proposals")
            .json(&a_proposal("Website replatform"))
            .await
            .json();
        assert_eq!(created["status"], "draft");
        let pid = created["pid"].as_str().expect("pid").to_string();

        // Promote straight from draft is refused (pipeline order).
        assert_eq!(
            request.post(&format!("/api/proposals/{pid}/promote")).await.status_code(),
            422
        );

        // Walk the pipeline.
        for (action, expected) in [
            ("submit", "submitted"),
            ("review", "in_review"),
            ("approve", "approved"),
        ] {
            let stepped: Value = request
                .post(&format!("/api/proposals/{pid}/{action}"))
                .await
                .json();
            assert_eq!(stepped["status"], expected, "{action}");
        }
        // Submitted/approved proposals are no longer editable.
        let edit = request
            .put(&format!("/api/proposals/{pid}"))
            .json(&a_proposal("Renamed"))
            .await;
        assert_eq!(edit.status_code(), 422);

        // Promote mints the work item in `projects`.
        let promoted: Value = request
            .post(&format!("/api/proposals/{pid}/promote"))
            .await
            .json();
        assert_eq!(promoted["status"], "promoted");
        let item_pid = promoted["work_item_pid"].as_str().expect("work item pid");
        let item: Value = request.get(&format!("/api/projects/{item_pid}")).await.json();
        assert_eq!(item["name"], "Website replatform");
        assert_eq!(item["kind"], "Project");

        // Promoting twice is refused; the proposal records the mint.
        assert_eq!(
            request.post(&format!("/api/proposals/{pid}/promote")).await.status_code(),
            422
        );
        let final_state: Value = request.get(&format!("/api/proposals/{pid}")).await.json();
        assert_eq!(
            final_state["promoted_work_item_pid"].as_str(),
            Some(item_pid)
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Duplicate-demand detection at intake: a proposal whose title matches
// a live work item (and a sibling proposal) is flagged before funding.
async fn duplicate_demand_is_flagged_at_intake() {
    request::<App, _, _>(|request, _ctx| async move {
        request
            .post("/api/projects")
            .json(&json!({ "kind": "Project", "name": "Data warehouse migration" }))
            .await
            .assert_status_ok();
        let first: Value = request
            .post("/api/proposals")
            .json(&a_proposal("Data warehouse migration"))
            .await
            .json();
        let second: Value = request
            .post("/api/proposals")
            .json(&a_proposal("Data warehouse migration"))
            .await
            .json();
        let hits: Value = request
            .get(&format!(
                "/api/proposals/{}/duplicates",
                second["pid"].as_str().unwrap()
            ))
            .await
            .json();
        let hits = hits.as_array().expect("hit list");
        assert!(
            hits.iter().any(|h| h["source"] == "work_item"),
            "the live work item is flagged: {hits:?}"
        );
        assert!(
            hits.iter()
                .any(|h| h["source"] == "proposal"
                    && h["pid"] == first["pid"]),
            "the sibling proposal is flagged: {hits:?}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The gate journey is strictly ordered: approvals advance the stage,
// hold does not, skipping refuses naming the expected gate, and the
// summary + review list expose stage / next_gate.
async fn gate_journey_advances_stage_in_order() {
    request::<App, _, _>(|request, _ctx| async move {
        let item: Value = request
            .post("/api/projects")
            .json(&json!({ "kind": "Project", "name": "Gated delivery" }))
            .await
            .json();
        let pid = item["pid"].as_str().expect("pid").to_string();

        // Skipping ahead refuses and names the expected gate.
        let skipped = request
            .post(&format!("/api/projects/{pid}/gate-reviews"))
            .json(&json!({ "gate": "g2_definition", "decision": "approved" }))
            .await;
        assert_eq!(skipped.status_code(), 422);

        // g0 approved ⇒ stage g0; a hold at g1 records but stays g0.
        let g0: Value = request
            .post(&format!("/api/projects/{pid}/gate-reviews"))
            .json(&json!({
                "gate": "g0_concept", "decision": "approved",
                "approver_ref": format!("worker:{}", uuid::Uuid::new_v4()),
            }))
            .await
            .json();
        assert_eq!(g0["stage"], "g0_concept");
        let held: Value = request
            .post(&format!("/api/projects/{pid}/gate-reviews"))
            .json(&json!({ "gate": "g1_feasibility", "decision": "hold",
                            "conditions": "budget confirmation needed" }))
            .await
            .json();
        assert_eq!(held["stage"], "g0_concept", "hold does not advance");

        let listed: Value = request
            .get(&format!("/api/projects/{pid}/gate-reviews"))
            .await
            .json();
        assert_eq!(listed["stage"], "g0_concept");
        assert_eq!(listed["next_gate"], "g1_feasibility");
        assert_eq!(listed["reviews"].as_array().map(Vec::len), Some(2));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Risks: scoring bounds enforce 1–5, the list derives exposure
// (highest first), and escalation materialises an open risk.
async fn risks_score_and_escalate() {
    request::<App, _, _>(|request, _ctx| async move {
        let item: Value = request
            .post("/api/programs")
            .json(&json!({ "kind": "Program", "name": "Risky programme" }))
            .await
            .json();
        let pid = item["pid"].as_str().expect("pid").to_string();

        let out_of_range = request
            .post(&format!("/api/programs/{pid}/risks"))
            .json(&json!({ "title": "Bad", "probability": 9, "impact": 3 }))
            .await;
        assert_eq!(out_of_range.status_code(), 422);

        let low: Value = request
            .post(&format!("/api/programs/{pid}/risks"))
            .json(&json!({ "title": "Minor slip", "probability": 2, "impact": 2 }))
            .await
            .json();
        let high: Value = request
            .post(&format!("/api/programs/{pid}/risks"))
            .json(&json!({ "title": "Vendor collapse", "probability": 4, "impact": 5,
                            "mitigation": "second-source the contract" }))
            .await
            .json();
        let listed: Value = request.get(&format!("/api/programs/{pid}/risks")).await.json();
        let listed = listed.as_array().expect("risk list");
        assert_eq!(listed[0]["exposure"], 20, "highest exposure first");
        assert_eq!(listed[1]["exposure"], 4);
        assert_eq!(listed[0]["pid"], high["pid"]);

        // Escalate the big one; a second escalation refuses.
        let escalate = format!(
            "/api/programs/{pid}/risks/{}/escalate",
            high["pid"].as_str().unwrap()
        );
        let materialised: Value = request.post(&escalate).await.json();
        assert_eq!(materialised["status"], "materialised");
        assert_eq!(request.post(&escalate).await.status_code(), 422);
        let low_pid = low["pid"].as_str().unwrap();
        let closed: Value = request
            .put(&format!("/api/programs/{pid}/risks/{low_pid}"))
            .json(&json!({ "status": "closed" }))
            .await
            .json();
        assert_eq!(closed["status"], "closed");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Budgets: minor-unit arithmetic, per-currency variance, currency
// shape validation, and the governance summary tying it together.
async fn budgets_track_variance_and_the_summary_aggregates() {
    request::<App, _, _>(|request, _ctx| async move {
        let item: Value = request
            .post("/api/portfolios")
            .json(&json!({ "kind": "Portfolio", "name": "Change portfolio" }))
            .await
            .json();
        let pid = item["pid"].as_str().expect("pid").to_string();

        let bad_currency = request
            .post(&format!("/api/portfolios/{pid}/budget-lines"))
            .json(&json!({ "category": "capex", "description": "x",
                            "currency": "pounds", "planned_minor": 100 }))
            .await;
        assert_eq!(bad_currency.status_code(), 422);

        let line: Value = request
            .post(&format!("/api/portfolios/{pid}/budget-lines"))
            .json(&json!({ "category": "capex", "description": "Platform build",
                            "currency": "GBP", "planned_minor": 10_000_000_i64 }))
            .await
            .json();
        let line_pid = line["pid"].as_str().expect("line pid").to_string();
        for _ in 0..2 {
            request
                .post(&format!("/api/portfolios/{pid}/budget-lines/{line_pid}/actual"))
                .json(&json!({ "amount_minor": 2_500_000_i64, "note": "invoice" }))
                .await
                .assert_status_ok();
        }
        let listed: Value = request
            .get(&format!("/api/portfolios/{pid}/budget-lines"))
            .await
            .json();
        assert_eq!(listed["totals"][0]["currency"], "GBP");
        assert_eq!(listed["totals"][0]["planned_minor"], 10_000_000_i64);
        assert_eq!(listed["totals"][0]["actual_minor"], 5_000_000_i64);
        assert_eq!(listed["totals"][0]["variance_minor"], 5_000_000_i64);

        // One risk + one gate approval, then the summary aggregates.
        request
            .post(&format!("/api/portfolios/{pid}/risks"))
            .json(&json!({ "title": "Funding risk", "probability": 3, "impact": 4 }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/portfolios/{pid}/gate-reviews"))
            .json(&json!({ "gate": "g0_concept", "decision": "approved_with_conditions" }))
            .await
            .assert_status_ok();
        let summary: Value = request
            .get(&format!("/api/portfolios/{pid}/governance"))
            .await
            .json();
        assert_eq!(summary["stage"], "g0_concept");
        assert_eq!(summary["next_gate"], "g1_feasibility");
        assert_eq!(summary["gate_reviews"], 1);
        assert_eq!(summary["risks"]["open"], 1);
        assert_eq!(summary["risks"]["max_exposure"], 12);
        assert_eq!(summary["budget"][0]["variance_minor"], 5_000_000_i64);
    })
    .await;
}
