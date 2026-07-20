//! The instance-layer round-trip: enrol on a template, the lifecycle
//! (hold / close / review cadence / urgency escalation), steps, care
//! team, and the derived caseload / overdue-reviews / cohort /
//! care-team-load views.

use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded pathway, the whole instance surface
async fn instance_layer_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        // A chronic-condition template (PrimaryCare).
        let template: Value = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": "Type 2 diabetes management",
                "care_setting": "PrimaryCare",
                "condition_codes": [{"system": "Icd10", "code": "E11"}],
            }))
            .await
            .json();
        let pathway = template["pid"].as_str().unwrap().to_string();
        let subject = format!("person:{}", uuid::Uuid::new_v4());
        let worker = format!("worker:{}", uuid::Uuid::new_v4());

        // ── Enrol (bad subject refused, then valid with two steps).
        assert_eq!(
            request
                .post(&format!("/api/care-pathways/{pathway}/instances"))
                .json(&json!({ "subject_ref": "nobody" }))
                .await
                .status_code(),
            422,
            "subject must be a person URN"
        );
        let instance: Value = request
            .post(&format!("/api/care-pathways/{pathway}/instances"))
            .json(&json!({
                "subject_ref": subject,
                "next_review_on": "2020-01-01",
                "steps": ["HbA1c test", "Foot check"],
            }))
            .await
            .json();
        let i_pid = instance["pid"].as_str().unwrap().to_string();
        assert_eq!(instance["status"], "active");
        assert_eq!(instance["urgency"], "routine");

        // ── Steps: fetch, complete one.
        let detail: Value = request.get(&format!("/api/instances/{i_pid}")).await.json();
        assert_eq!(detail["steps"].as_array().unwrap().len(), 2);
        let step_pid = detail["steps"][0]["pid"].as_str().unwrap().to_string();
        let done: Value = request
            .post(&format!("/api/instances/{i_pid}/steps/{step_pid}/complete"))
            .await
            .json();
        assert_eq!(done["done"], true);
        assert!(done["done_on"].is_string());

        // ── Care team: add a GP; duplicate role refused.
        request
            .post(&format!("/api/instances/{i_pid}/team"))
            .json(&json!({ "member_ref": worker, "role": "gp" }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/instances/{i_pid}/team"))
                .json(&json!({ "member_ref": worker, "role": "gp" }))
                .await
                .status_code(),
            422,
            "duplicate (member, role) refused"
        );

        // ── Urgency escalation records an escalation event.
        request
            .post(&format!("/api/instances/{i_pid}/urgency"))
            .json(&json!({ "to": "urgent", "note": "hypo episode" }))
            .await
            .assert_status_ok();

        // ── Overdue reviews: next_review_on is 2020 ⇒ overdue.
        let overdue: Value = request.get("/api/instances/overdue-reviews").await.json();
        assert!(overdue["overdue"].as_array().unwrap().iter()
            .any(|o| o["pid"] == json!(i_pid)));

        // ── Record a review ⇒ reschedules, drops off overdue.
        request
            .post(&format!("/api/instances/{i_pid}/review"))
            .json(&json!({ "next_review_on": "2099-01-01", "note": "stable" }))
            .await
            .assert_status_ok();
        let overdue: Value = request.get("/api/instances/overdue-reviews").await.json();
        assert!(!overdue["overdue"].as_array().unwrap().iter()
            .any(|o| o["pid"] == json!(i_pid)), "no longer overdue");

        // ── Caseload: one open, urgent, in PrimaryCare.
        let caseload: Value = request.get("/api/instances/caseload").await.json();
        assert_eq!(caseload["open"], 1);
        assert_eq!(caseload["by_setting"]["PrimaryCare"], 1);
        assert_eq!(caseload["by_urgency"]["urgent"], 1);
        assert_eq!(caseload["urgent_or_emergency"], 1);

        // ── Cohort on the pathway.
        let cohort: Value = request.get(&format!("/api/care-pathways/{pathway}/cohort")).await.json();
        assert_eq!(cohort["instances"], 1);
        assert_eq!(cohort["by_status"]["active"], 1);
        assert_eq!(cohort["step_completion"]["done"], 1);
        assert_eq!(cohort["step_completion"]["total"], 2);

        // ── Care-team load: the GP carries one open instance.
        let load: Value = request.get("/api/instances/care-team-load").await.json();
        let member = load["members"].as_array().unwrap().iter()
            .find(|m| m["member_ref"] == json!(worker)).expect("gp row").clone();
        assert_eq!(member["open_instances"], 1);

        // ── Measures: numeric HbA1c reading, then a second lower.
        assert_eq!(
            request
                .post(&format!("/api/instances/{i_pid}/measures"))
                .json(&json!({ "name": "HbA1c" }))
                .await
                .status_code(),
            422,
            "a value is required"
        );
        request
            .post(&format!("/api/instances/{i_pid}/measures"))
            .json(&json!({ "name": "HbA1c", "value_numeric": 64.0, "unit": "mmol/mol",
                            "recorded_on": "2026-06-01" }))
            .await
            .assert_status_ok();
        request
            .post(&format!("/api/instances/{i_pid}/measures"))
            .json(&json!({ "name": "HbA1c", "value_numeric": 52.0, "unit": "mmol/mol",
                            "recorded_on": "2026-07-01" }))
            .await
            .assert_status_ok();
        let detail: Value = request.get(&format!("/api/instances/{i_pid}")).await.json();
        assert_eq!(detail["measures"].as_array().unwrap().len(), 2);

        // ── Lifecycle: hold → active → complete; then no review.
        request
            .post(&format!("/api/instances/{i_pid}/status"))
            .json(&json!({ "to": "on_hold" }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/instances/{i_pid}/status"))
                .json(&json!({ "to": "on_hold" }))
                .await
                .status_code(),
            422,
            "no self-loop"
        );
        // A bogus outcome is refused; a valid one is recorded at close.
        assert_eq!(
            request
                .post(&format!("/api/instances/{i_pid}/status"))
                .json(&json!({ "to": "completed", "outcome": "vibes" }))
                .await
                .status_code(),
            422,
            "unknown outcome refused"
        );
        let closed: Value = request
            .post(&format!("/api/instances/{i_pid}/status"))
            .json(&json!({ "to": "completed", "reason": "target met", "outcome": "improved" }))
            .await
            .json();
        assert!(closed["closed_on"].is_string());
        assert_eq!(closed["outcome"], "improved");
        assert_eq!(
            request
                .post(&format!("/api/instances/{i_pid}/status"))
                .json(&json!({ "to": "active" }))
                .await
                .status_code(),
            422,
            "completed is terminal"
        );
        assert_eq!(
            request
                .post(&format!("/api/instances/{i_pid}/review"))
                .json(&json!({ "next_review_on": "2099-06-01" }))
                .await
                .status_code(),
            422,
            "a closed instance is not reviewed"
        );
        // Caseload now empty.
        let caseload: Value = request.get("/api/instances/caseload").await.json();
        assert_eq!(caseload["open"], 0);

        // ── Outcomes: one closed instance, outcome improved; the
        // latest HbA1c (52) is the measure average.
        let outcomes: Value = request
            .get(&format!("/api/care-pathways/{pathway}/outcomes"))
            .await
            .json();
        assert_eq!(outcomes["closed_instances"], 1);
        assert_eq!(outcomes["outcome_distribution"]["improved"], 1);
        let hba1c = outcomes["measure_summary"].as_array().unwrap().iter()
            .find(|m| m["name"] == "HbA1c").expect("HbA1c summary").clone();
        assert_eq!(hba1c["instances_with_measure"], 1);
        assert!((hba1c["latest_value_average"].as_f64().unwrap() - 52.0).abs() < 1e-9);
    })
    .await;
}
