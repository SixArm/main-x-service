//! The engagement round-trip: declared stakeholder typing + the
//! power–interest grid, recorded sentiment, cadence / workload /
//! funnel / member-health / consent-by-account views, the partnership
//! lifecycle, memberships + renewals, and working groups.

use contact_relationship_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, a_worker, seed_pipeline};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // one seeded estate, the whole surface
async fn engagement_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        // ── Accounts + contacts.
        let account: Value = request
            .post("/api/accounts")
            .json(&json!({ "organization_ref": format!("organization:{}", uuid::Uuid::new_v4()),
                            "display_name": "Meridian University" }))
            .await
            .json();
        let account_pid = account["pid"].as_str().unwrap().to_string();
        let contact: Value = request
            .post("/api/contacts")
            .json(&json!({ "person_ref": a_person(), "display_name": "Prof Reyes",
                            "account_pid": account_pid }))
            .await
            .json();
        let contact_pid = contact["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/contacts/{contact_pid}/consent"))
            .json(&json!({ "action": "granted", "source": "web form" }))
            .await
            .assert_status_ok();

        // ── Declared stakeholder typing (validated; grid 1–5).
        assert_eq!(
            request
                .put(&format!("/api/contacts/{contact_pid}/stakeholder"))
                .json(&json!({ "role": "overlord" }))
                .await
                .status_code(),
            422,
            "unknown role refused"
        );
        assert_eq!(
            request
                .put(&format!("/api/contacts/{contact_pid}/stakeholder"))
                .json(&json!({ "role": "partner", "influence": 9 }))
                .await
                .status_code(),
            422,
            "grid scores are 1-5"
        );
        request
            .put(&format!("/api/contacts/{contact_pid}/stakeholder"))
            .json(&json!({ "role": "partner", "influence": 4, "interest": 5 }))
            .await
            .assert_status_ok();
        request
            .put(&format!("/api/accounts/{account_pid}/stakeholder"))
            .json(&json!({ "role": "partner" }))
            .await
            .assert_status_ok();

        // ── An activity with recorded sentiment + a due follow-up task.
        assert_eq!(
            request
                .post("/api/activities")
                .json(&json!({ "subject_kind": "contact", "subject_pid": contact_pid,
                                "kind": "meeting", "summary": "Kickoff",
                                "sentiment": "sideways" }))
                .await
                .status_code(),
            422,
            "unknown sentiment refused"
        );
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "contact", "subject_pid": contact_pid,
                            "kind": "meeting", "summary": "Kickoff",
                            "actor_ref": a_worker(), "sentiment": "positive" }))
            .await
            .assert_status_ok();
        request
            .post("/api/activities")
            .json(&json!({ "subject_kind": "contact", "subject_pid": contact_pid,
                            "kind": "task", "summary": "renewal: MoU 2027",
                            "due_on": "2026-08-15" }))
            .await
            .assert_status_ok();

        // ── Cadence: the touched contact is not in the untouched list;
        // a fresh silent account is (ages from creation with days=0 →
        // not over threshold 0? use threshold high). Create a silent
        // account to exercise the account list at threshold 0... days
        // since creation is 0 today, so assert the shape instead.
        let cadence: Value = request.get("/api/insights/cadence?days=1").await.json();
        assert_eq!(cadence["untouched_contacts"].as_array().unwrap().len(), 0);
        assert_eq!(cadence["contacts_without_next_touch"], 0, "the renewal task counts");

        // ── Engagement workload: kinds + recorded sentiment counted.
        let engagement: Value = request.get("/api/insights/engagement").await.json();
        assert_eq!(engagement["touches"], 2);
        assert_eq!(engagement["per_kind"]["meeting"], 1);
        assert_eq!(engagement["sentiment"]["positive"], 1);
        assert_eq!(engagement["sentiment"]["unrecorded"], 1);

        // ── Followups kind filter (the renewals convention).
        let renewals: Value = request.get("/api/insights/followups?kind=task").await.json();
        assert_eq!(renewals["upcoming_30d"].as_array().unwrap().len(), 1);
        let calls: Value = request.get("/api/insights/followups?kind=call").await.json();
        assert_eq!(calls["upcoming_30d"].as_array().unwrap().len(), 0);

        // ── Funnel over a seeded pipeline: two deals created, one
        // moved to Proposal — entered counts + honest conversion.
        let (pipeline_pid, stages) = seed_pipeline(&request).await;
        for name in ["Deal A", "Deal B"] {
            let deal: Value = request
                .post("/api/deals")
                .json(&json!({ "name": name, "pipeline_pid": pipeline_pid,
                                "amount_minor": 100_000_i64, "currency": "GBP" }))
                .await
                .json();
            if name == "Deal A" {
                let pid = deal["pid"].as_str().unwrap();
                request
                    .post(&format!("/api/deals/{pid}/stage"))
                    .json(&json!({ "stage_pid": stages[1] }))
                    .await
                    .assert_status_ok();
            }
        }
        let funnel: Value = request
            .get(&format!("/api/insights/funnel?pipeline={pipeline_pid}"))
            .await
            .json();
        let rows = funnel["stages"].as_array().unwrap();
        assert_eq!(rows[0]["entered"], 2);
        assert_eq!(rows[1]["entered"], 1);
        assert_eq!(rows[1]["conversion_from_previous"]["numerator"], 1);
        assert_eq!(rows[1]["conversion_from_previous"]["denominator"], 2);

        // ── Partnership lifecycle: forward-only + retire.
        let partnership: Value = request
            .post(&format!("/api/accounts/{account_pid}/partnerships"))
            .json(&json!({ "kind": "university", "summary": "Joint ML lab" }))
            .await
            .json();
        let p_pid = partnership["pid"].as_str().unwrap().to_string();
        assert_eq!(partnership["stage"], "scouting");
        assert_eq!(
            request
                .post(&format!("/api/partnerships/{p_pid}/stage"))
                .json(&json!({ "to": "scaled" }))
                .await
                .status_code(),
            422,
            "no skipping stages"
        );
        request
            .post(&format!("/api/partnerships/{p_pid}/stage"))
            .json(&json!({ "to": "pilot" }))
            .await
            .assert_status_ok();
        let register: Value = request.get("/api/insights/partnerships").await.json();
        assert_eq!(register["by_stage"]["pilot"], 1);
        assert_eq!(register["register"][0]["account"], "Meridian University");

        // ── Membership: upsert + renewals-due view.
        request
            .put(&format!("/api/accounts/{account_pid}/membership"))
            .json(&json!({ "joined_on": "2024-01-01", "renewal_on": "2026-08-01" }))
            .await
            .assert_status_ok();
        let memberships: Value = request.get("/api/insights/memberships?days=30").await.json();
        assert_eq!(memberships["renewals_due"].as_array().unwrap().len(), 1);
        assert_eq!(memberships["renewals_due"][0]["account"], "Meridian University");

        // ── Working group: roster + derived feed.
        let group: Value = request
            .post("/api/groups")
            .json(&json!({ "name": "Data Standards WG", "purpose": "Align schemas" }))
            .await
            .json();
        let g_pid = group["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/groups/{g_pid}/members"))
            .json(&json!({ "contact_pid": contact_pid }))
            .await
            .assert_status_ok();
        assert_eq!(
            request
                .post(&format!("/api/groups/{g_pid}/members"))
                .json(&json!({ "contact_pid": contact_pid }))
                .await
                .status_code(),
            422,
            "duplicate membership refused"
        );
        let detail: Value = request.get(&format!("/api/groups/{g_pid}")).await.json();
        assert_eq!(detail["roster"][0]["display_name"], "Prof Reyes");
        assert_eq!(detail["feed"].as_array().unwrap().len(), 2);

        // ── Stakeholder register + grid.
        let stakeholders: Value = request.get("/api/insights/stakeholders").await.json();
        assert_eq!(stakeholders["by_role"]["partner"].as_array().unwrap().len(), 1);
        assert_eq!(stakeholders["grid"]["p4i5"], 1);
        assert_eq!(stakeholders["account_roles"][0]["role"], "partner");

        // ── Member health + consent-by-account.
        let members: Value = request.get("/api/insights/members?days=365").await.json();
        let row = members["accounts"].as_array().unwrap().iter()
            .find(|a| a["display_name"] == "Meridian University")
            .expect("account row").clone();
        assert_eq!(row["contacts"], 1);
        assert_eq!(row["silent"], false);
        assert_eq!(row["membership"]["status"], "active");
        let consent: Value = request.get("/api/insights/consent-by-account").await.json();
        let row = consent["accounts"].as_array().unwrap().iter()
            .find(|a| a["display_name"] == "Meridian University")
            .expect("consent row").clone();
        assert_eq!(row["consent_coverage"]["granted"], 1);
    })
    .await;
}
