//! The insight-view round-trip: stale-deal aging, follow-ups,
//! pipeline hygiene, the executive pack, forecast trends, the SLA
//! register, and the DPO view — over a seeded estate.

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, a_worker, seed_contact, seed_pipeline};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded estate, seven views
async fn insight_views_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let (pipeline_pid, stages) = seed_pipeline!(&request).await;
        let contact_pid = seed_contact!(&request, "Prime Contact").await;

        // A duplicate CRM row for the same person URN (hygiene case).
        let shared_person = a_person();
        for name in ["Row One", "Row Two"] {
            request
                .post("/api/contacts")
                .json(&json!({ "person_ref": shared_person, "display_name": name }))
                .await
                .assert_status_ok();
        }

        // Deals: one moved (fresh stage clock), one never moved with
        // no amount and no expected close (hygiene findings), one won.
        let moved: Value = request
            .post("/api/deals")
            .json(&json!({ "name": "Moved deal", "pipeline_pid": pipeline_pid,
                            "amount_minor": 400_000_i64, "currency": "GBP" }))
            .await
            .json();
        let moved_pid = moved["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/deals/{moved_pid}/stage"))
            .json(&json!({ "stage_pid": stages[1] }))
            .await
            .assert_status_ok();
        request
            .post("/api/deals")
            .json(
                &json!({ "name": "Neglected deal", "pipeline_pid": pipeline_pid,
                            "amount_minor": 0_i64, "currency": "GBP" }),
            )
            .await
            .assert_status_ok();
        let won: Value = request
            .post("/api/deals")
            .json(&json!({ "name": "Won deal", "pipeline_pid": pipeline_pid,
                            "amount_minor": 250_000_i64, "currency": "GBP" }))
            .await
            .json();
        let won_pid = won["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/deals/{won_pid}/stage"))
            .json(&json!({ "stage_pid": stages[2] }))
            .await
            .assert_status_ok();

        // Activities: an overdue follow-up on the moved deal.
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "deal", "subject_pid": moved_pid,
                            "kind": "call", "summary": "Chase the proposal",
                            "actor_ref": a_worker(), "due_on": "2026-07-01" }))
            .await
            .assert_status_ok();

        // Support: a policy then a ticket whose deadlines are derived.
        request
            .post("/api/sla-policies")
            .json(&json!({ "priority": "high", "first_response_minutes": 1,
                            "resolution_minutes": 2 }))
            .await
            .assert_status_ok();
        request
            .post("/api/tickets")
            .json(&json!({ "title": "Login broken", "priority": "high",
                            "contact_pid": contact_pid,
                            "assignee_ref": a_worker() }))
            .await
            .assert_status_ok();

        // A forecast snapshot for the trend series.
        request
            .post("/api/forecast/snapshot")
            .await
            .assert_status_ok();

        // One withdrawal for the DPO window.
        request
            .post(&format!("/api/contacts/{contact_pid}/consent"))
            .json(&json!({ "action": "withdrawn", "source": "email link" }))
            .await
            .assert_status_ok();

        // ── Stale deals: both open deals reported; the never-moved one
        // ages from creation (0 days today), so nothing is stale yet.
        let stale: Value = request.get("/api/insights/stale-deals?days=1").await.json();
        assert_eq!(stale["open_deals"], 2);
        assert_eq!(stale["stale_deals"], 0);
        assert!(
            stale["derivation"]
                .as_str()
                .unwrap()
                .contains("deal_stage_changed")
        );

        // ── Follow-ups: the overdue call shows with its age.
        let followups: Value = request.get("/api/insights/followups").await.json();
        assert_eq!(followups["overdue"].as_array().unwrap().len(), 1);
        assert!(followups["overdue"][0]["overdue_days"].as_i64().unwrap() > 0);
        assert_eq!(followups["overdue"][0]["summary"], "Chase the proposal");

        // ── Hygiene: the neglected deal fires amount + close + activity
        // rules; the moved deal only the recent-activity rule is spared
        // (it has an activity logged today via occurred_at default).
        let hygiene: Value = request
            .get("/api/insights/pipeline-hygiene?days=1")
            .await
            .json();
        let rules: Vec<&str> = hygiene["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|f| f["rule"].as_str())
            .collect();
        assert!(rules.contains(&"open_deal_without_amount"));
        assert!(rules.contains(&"open_deal_without_expected_close"));
        assert!(rules.contains(&"open_deal_without_recent_activity"));

        // ── Executive pack: the won deal + its value; the withdrawal.
        let pack: Value = request.get("/api/insights/executive").await.json();
        assert_eq!(pack["deals_won"], 1);
        assert_eq!(pack["deals_lost"], 0);
        assert_eq!(pack["won_value_by_currency_minor"]["GBP"], 250_000);
        assert_eq!(pack["tickets_opened"], 1);
        assert_eq!(pack["consent_withdrawals"], 1);
        assert!(pack["activities_logged"].as_u64().unwrap() >= 1);

        // ── Forecast trends: the stored snapshot only.
        let trends: Value = request.get("/api/insights/forecast-trends").await.json();
        assert_eq!(trends["series"].as_array().unwrap().len(), 1);
        assert!(
            trends["note"]
                .as_str()
                .unwrap()
                .contains("no interpolated history")
        );

        // ── SLA register: the 1-minute first-response deadline may or
        // may not have passed yet; workload always reports the open
        // ticket under its assignee.
        let sla: Value = request.get("/api/insights/sla").await.json();
        let workload = sla["workload"].as_array().unwrap();
        assert_eq!(workload.len(), 1);
        assert_eq!(workload[0]["open"], 1);

        // ── DPO: coverage counts, the withdrawal, the duplicate pair.
        let dpo: Value = request.get("/api/insights/dpo").await.json();
        assert_eq!(dpo["contacts"], 3);
        assert_eq!(dpo["consent_coverage"]["withdrawn"], 1);
        assert_eq!(dpo["withdrawals_in_window"], 1);
        assert_eq!(dpo["consent_events_by_source"]["web form"], 1);
        let dupes = dpo["duplicate_person_refs"].as_array().unwrap();
        assert_eq!(dupes.len(), 1);
        assert_eq!(dupes[0]["contacts"].as_array().unwrap().len(), 2);

        // ── ETag replay on one view.
        let first = request.get("/api/insights/dpo").await;
        let etag = first
            .headers()
            .get("etag")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let replay = request
            .get("/api/insights/dpo")
            .add_header("if-none-match", etag)
            .await;
        assert_eq!(replay.status_code(), 304);
    })
    .await;
}
