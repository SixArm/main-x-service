//! The patient journey over the live routes (PF-T17): bed request →
//! ranked eligibility → allocate → admit → SAFER / Red2Green →
//! discharge-ready gate → discharge → the deep-clean cycle — plus the
//! **double-placement race** (two concurrent admits to one bed ⇒
//! exactly one succeeds; PF-D9's `FOR UPDATE` pin).

use loco_rs::testing::prelude::*;
use patient_flow_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

use super::{a_person, seed_ward};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
#[allow(clippy::too_many_lines)] // the full journey is one narrative
async fn full_journey_request_to_deep_clean() {
    request::<App, _, _>(|request, _ctx| async move {
        let (ward_pid, _bed_pids) = seed_ward(&request, "inpatient", Some("respiratory"), 2).await;
        let person = a_person();

        // Queue a bed request; the ranked eligible list offers our beds.
        let bed_request: Value = request
            .post("/api/bed-requests")
            .json(&json!({
                "person_ref": person, "origin": "ed", "priority": "urgent",
                "target_ward_pid": ward_pid, "requirements": { "oxygen": true },
            }))
            .await
            .json();
        let request_pid = bed_request["pid"]
            .as_str()
            .expect("request pid")
            .to_string();
        let eligible: Value = request
            .get(&format!("/api/bed-requests/{request_pid}/eligible"))
            .await
            .json();
        assert_eq!(
            eligible.as_array().map(Vec::len),
            Some(2),
            "both beds eligible"
        );

        // Allocate the first bed: request → allocated, bed → reserved.
        let chosen = eligible[0]["bed_pid"]
            .as_str()
            .expect("bed pid")
            .to_string();
        let allocated: Value = request
            .post(&format!("/api/bed-requests/{request_pid}/allocate"))
            .json(&json!({ "bed_pid": chosen }))
            .await
            .json();
        assert_eq!(allocated["status"], "allocated");
        let bed: Value = request.get(&format!("/api/beds/{chosen}")).await.json();
        assert_eq!(bed["state"], "reserved");

        // Admit into the reserved bed, fulfilling the request. No EDD
        // yet ⇒ the SAFER nudge flags it.
        let stay: Value = request
            .post("/api/stays")
            .json(&json!({
                "person_ref": person, "display_name": "Test Patient",
                "source": "ed", "bed_pid": chosen, "bed_request_pid": request_pid,
            }))
            .await
            .json();
        assert_eq!(stay["edd_missing"], true, "missing EDD is flagged on admit");
        let stay_pid = stay["pid"].as_str().expect("stay pid").to_string();

        // Discharge-ready is gated on EDD + CCD (SAFER "A").
        let premature = request
            .post(&format!("/api/stays/{stay_pid}/discharge-ready"))
            .json(&json!({ "pathway": "p1" }))
            .await;
        assert_eq!(premature.status_code(), 422, "no EDD/CCD ⇒ not ready");
        request
            .put(&format!("/api/stays/{stay_pid}"))
            .json(&json!({ "edd": "2026-07-20", "ccd_met": true, "senior_review_now": true }))
            .await
            .assert_status_ok();

        // Red2Green: a red day with a coded reason; >2 reasons is 422.
        request
            .post(&format!("/api/stays/{stay_pid}/red-green"))
            .json(&json!({ "classification": "red", "delay_reasons": ["awaiting_diagnostics"] }))
            .await
            .assert_status_ok();
        let too_many = request
            .post(&format!("/api/stays/{stay_pid}/red-green"))
            .json(&json!({ "classification": "red",
                "delay_reasons": ["awaiting_diagnostics", "awaiting_pharmacy", "other"] }))
            .await;
        assert_eq!(too_many.status_code(), 422);

        // ── The stitched-journey timeline contract. The stay has one
        // red day so far and no discharge, so it reads as a running
        // clock with no value-adding time yet — and, crucially, says
        // that is a *classified* zero rather than an unfilled board.
        let timeline: Value = request
            .get(&format!("/api/stays/{stay_pid}/time-analysis"))
            .await
            .json();
        for key in [
            "lead_time_ms",
            "value_time_ms",
            "span_days",
            "classified_days",
        ] {
            assert!(timeline[key].is_i64(), "missing {key} in {timeline}");
        }
        assert!(timeline["clock"]["start_ms"].is_i64());
        assert!(timeline["clock"]["stop_ms"].is_i64());
        assert_eq!(timeline["clock"]["start_source"], "admitted_at");
        assert_eq!(timeline["clock"]["stop_source"], "as_of", "still admitted");
        assert_eq!(timeline["clock"]["running"], true);
        assert_eq!(timeline["classified_days"], 1);
        assert_eq!(timeline["green_days"], 0);
        assert_eq!(timeline["value_time_ms"], 0, "a red day adds no value time");
        assert_eq!(
            timeline["confidence"], "classified",
            "the board was filled in — this zero is a finding, not a gap"
        );

        // A green day would be value-adding time, but `red-green` always
        // classifies *today*, so a second day cannot be added within one
        // test run — the green arithmetic is covered by the pure tests
        // in `flow::journey`, and posting one here would overwrite the
        // red day this test already asserts.

        // Raise a transmissible flag, then discharge: the vacated bed
        // owes a deep clean, and a routine clean-complete is refused.
        request
            .post(&format!("/api/stays/{stay_pid}/infection-flags"))
            .json(
                &json!({ "precaution": "droplet", "organism": "covid-19", "status": "suspected" }),
            )
            .await
            .assert_status_ok();
        let ready: Value = request
            .post(&format!("/api/stays/{stay_pid}/discharge-ready"))
            .json(&json!({ "pathway": "p1" }))
            .await
            .json();
        assert_eq!(ready["status"], "discharge_ready");
        let done: Value = request
            .post(&format!("/api/stays/{stay_pid}/discharge"))
            .json(&json!({ "destination": "home_with_support" }))
            .await
            .json();
        assert_eq!(done["status"], "discharged");

        let vacated: Value = request.get(&format!("/api/beds/{chosen}")).await.json();
        assert_eq!(vacated["state"], "awaiting_clean");
        assert_eq!(vacated["deep_clean_required"], true);
        request
            .post(&format!("/api/beds/{chosen}/state"))
            .json(&json!({ "transition": "clean_start" }))
            .await
            .assert_status_ok();
        let routine = request
            .post(&format!("/api/beds/{chosen}/state"))
            .json(&json!({ "transition": "clean_complete" }))
            .await;
        assert_eq!(
            routine.status_code(),
            422,
            "routine clean refused while deep clean owed"
        );
        let deep: Value = request
            .post(&format!("/api/beds/{chosen}/state"))
            .json(&json!({ "transition": "clean_complete", "deep_clean_done": true }))
            .await
            .json();
        assert_eq!(deep["state"], "available");
        assert_eq!(deep["deep_clean_required"], false);

        // The stay detail records the whole narrative.
        let detail: Value = request.get(&format!("/api/stays/{stay_pid}")).await.json();
        assert_eq!(detail["stay"]["status"], "discharged");
        assert_eq!(
            detail["transfers"].as_array().map(Vec::len),
            Some(2),
            "admit + discharge moves"
        );
        assert_eq!(detail["red_green"][0]["classification"], "red");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The double-placement race (PF-D9): two concurrent admits to the same
// available bed produce exactly one 200 and one 422 — the `FOR UPDATE`
// row lock serialises the placement check.
async fn concurrent_admits_place_exactly_one_patient() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_, bed_pids) = seed_ward(&request, "inpatient", None, 1).await;
        let bed = bed_pids[0].clone();
        let admit = |person: String| {
            let request = &request;
            let bed = bed.clone();
            async move {
                request
                    .post("/api/stays")
                    .json(&json!({
                        "person_ref": person, "display_name": "Race Patient",
                        "source": "ed", "bed_pid": bed,
                    }))
                    .await
                    .status_code()
            }
        };
        let (a, b) = tokio::join!(admit(a_person()), admit(a_person()));
        let mut statuses = [a.as_u16(), b.as_u16()];
        statuses.sort_unstable();
        assert_eq!(statuses, [200, 422], "exactly one admit wins the bed");
    })
    .await;
}
