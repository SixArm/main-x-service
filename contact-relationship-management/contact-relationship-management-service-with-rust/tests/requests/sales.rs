//! The sales journey (CRM-R3–R5): capture → score → convert →
//! pipeline → won, with the forecast reflecting every move, plus the
//! unknown-pid 404 pins.

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_pipeline};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Lead: scored on capture with breakdown; the state machine refuses a
// skip to converted; conversion creates the contact + deal in-tx;
// the deal moves through stages to Won; the forecast follows.
async fn sales_journey_end_to_end() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline_pid, stage_pids) = seed_pipeline!(&request).await;
        // Capture a referral lead with a corporate domain: 20+10 = 30.
        let captured: Value = request
            .post("/api/leads")
            .json(&json!({
                "display_name": "Test Prospect",
                "source": "referral",
                "email": "prospect@initech.example",
            }))
            .await
            .json();
        let lead_pid = captured["pid"].as_str().unwrap().to_string();
        assert_eq!(captured["score"]["score"], 30);
        assert_eq!(captured["score"]["label"], "cold");
        let rules = captured["score"]["rules"].as_array().unwrap();
        assert_eq!(rules.len(), 8, "every rule listed in the breakdown");
        // Skip to converted is refused by the machine.
        let skip = request
            .post(&format!("/api/leads/{lead_pid}/status"))
            .json(&json!({ "to": "converted" }))
            .await;
        assert_eq!(skip.status_code(), 422);
        // Walk: contacted → qualified → converted (creates contact + deal).
        for to in ["contacted", "qualified"] {
            request
                .post(&format!("/api/leads/{lead_pid}/status"))
                .json(&json!({ "to": to }))
                .await
                .assert_status_ok();
        }
        let converted: Value = request
            .post(&format!("/api/leads/{lead_pid}/status"))
            .json(&json!({
                "to": "converted",
                "person_ref": a_person(),
                "deal": {
                    "name": "Prospect Deal", "pipeline_pid": pipeline_pid,
                    "amount_minor": 1_000_000, "currency": "GBP",
                },
            }))
            .await
            .json();
        let deal_pid = converted["deal_pid"]
            .as_str()
            .expect("deal opened")
            .to_string();
        assert!(converted["contact_pid"].is_string(), "contact created");
        // Forecast at the first stage (10%): 100,000.
        let forecast: Value = request.get("/api/forecast").await.json();
        assert_eq!(forecast["totals_minor"]["GBP"], 100_000);
        // Move to Proposal (50%): 500,000.
        request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stage_pids[1] }))
            .await
            .assert_status_ok();
        let forecast: Value = request.get("/api/forecast").await.json();
        assert_eq!(forecast["totals_minor"]["GBP"], 500_000);
        // Lost without a reason is refused; Won closes the deal.
        let lost_no_reason = request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stage_pids[3] }))
            .await;
        assert_eq!(lost_no_reason.status_code(), 422);
        request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stage_pids[2] }))
            .await
            .assert_status_ok();
        // Closed deals leave the forecast; further moves are refused.
        let forecast: Value = request.get("/api/forecast").await.json();
        assert!(forecast["totals_minor"].as_object().unwrap().is_empty());
        let move_closed = request
            .post(&format!("/api/deals/{deal_pid}/stage"))
            .json(&json!({ "stage_pid": stage_pids[1] }))
            .await;
        assert_eq!(move_closed.status_code(), 422, "closed deals are immutable");
        // Reasoned reopen returns it to an open stage.
        request
            .post(&format!("/api/deals/{deal_pid}/reopen"))
            .json(&json!({ "reason": "signature fell through" }))
            .await
            .assert_status_ok();
        let forecast: Value = request.get("/api/forecast").await.json();
        assert_eq!(
            forecast["totals_minor"]["GBP"], 500_000,
            "reopened at Proposal"
        );
        // The sales dashboard reports the honest win rate parts.
        let dashboard: Value = request.get("/api/dashboards/sales").await.json();
        assert_eq!(dashboard["win_rate"]["denominator"], 0);
        assert!(
            dashboard["win_rate"]["value"].is_null(),
            "0/0 is null, not 0%"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Unknown-pid reads are honest 404s (family lesson); a stage from a
// different pipeline is refused.
async fn contracts_404_and_pipeline_membership() {
    request::<App, _, _>(|request, _ctx| async move {
        let ghost = uuid::Uuid::new_v4();
        for path in [
            format!("/api/contacts/{ghost}"),
            format!("/api/leads/{ghost}"),
            format!("/api/tickets/{ghost}"),
            "/api/contacts/not-a-uuid".to_string(),
        ] {
            assert_eq!(request.get(&path).await.status_code(), 404, "{path}");
        }
        // A stage from another pipeline is refused on a move.
        let (pipeline_a, _stages_a) = seed_pipeline!(&request).await;
        let (_pipeline_b, stages_b) = seed_pipeline!(&request).await;
        let deal: Value = request
            .post("/api/deals")
            .json(&json!({
                "name": "Cross-pipeline", "pipeline_pid": pipeline_a,
                "amount_minor": 100, "currency": "GBP",
            }))
            .await
            .json();
        let cross = request
            .post(&format!(
                "/api/deals/{}/stage",
                deal["pid"].as_str().unwrap()
            ))
            .json(&json!({ "stage_pid": stages_b[1] }))
            .await;
        assert_eq!(
            cross.status_code(),
            422,
            "stage from another pipeline refused"
        );
    })
    .await;
}
