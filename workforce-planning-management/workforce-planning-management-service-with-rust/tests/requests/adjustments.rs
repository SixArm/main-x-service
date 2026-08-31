//! The reasonable-adjustments round-trip (WPM-R33 / WPM-D25): the
//! useful bit is required (barrier + impact + change), the lifecycle
//! decides with a note and notifies the employee, and the words stay
//! on the record — in writing, in the subject-access export.

use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use workforce_planning_management_service::app::App;

use super::{activate, an_org, seed_employee};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn adjustments_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let employee = seed_employee!(&request, &org, "RA-1", None).await;
        activate!(&request, &employee).await;

        // The useful bit is required — a request without the barrier
        // (or an unknown category) is refused.
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee}/adjustment-requests"))
                .json(&json!({ "category": "quieter_workspace", "barrier": " ",
                               "impact": "x", "adjustment": "y" }))
                .await
                .status_code(),
            422,
            "barrier is the point"
        );
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee}/adjustment-requests"))
                .json(&json!({ "category": "diagnosis", "barrier": "b",
                               "impact": "i", "adjustment": "a" }))
                .await
                .status_code(),
            422,
            "categories are practical, closed, and hold no label"
        );
        let created: Value = request
            .post(&format!("/api/employees/{employee}/adjustment-requests"))
            .json(&json!({
                "category": "quieter_workspace",
                "barrier": "Open-plan noise makes sustained focus hard",
                "impact": "Deep-work tasks take much longer in the afternoons",
                "adjustment": "A desk in the quiet corner, plus focus blocks",
            }))
            .await
            .json();
        let r_pid = created["pid"].as_str().unwrap().to_string();

        // The list carries the words verbatim (unmasked read) and the
        // read is audited.
        let listed: Value = request
            .get(&format!("/api/employees/{employee}/adjustment-requests"))
            .await
            .json();
        assert_eq!(listed[0]["status"], "requested");
        assert_eq!(listed[0]["words_withheld"], false);
        assert!(
            listed[0]["barrier"]
                .as_str()
                .unwrap()
                .contains("Open-plan noise")
        );
        let audits: Value = request.get("/api/audits/recent").await.json();
        assert!(
            serde_json::to_string(&audits)
                .unwrap()
                .contains("adjustments_read")
        );

        // The lifecycle: agree with a practical note, then in place;
        // a declined request cannot be revived.
        assert_eq!(
            request
                .post(&format!("/api/adjustment-requests/{r_pid}/status"))
                .json(&json!({ "to": "in_place" }))
                .await
                .status_code(),
            422,
            "agree first"
        );
        request
            .post(&format!("/api/adjustment-requests/{r_pid}/status"))
            .json(&json!({ "to": "agreed", "note": "Corner desk from Monday" }))
            .await
            .assert_status_ok();
        let in_place: Value = request
            .post(&format!("/api/adjustment-requests/{r_pid}/status"))
            .json(&json!({ "to": "in_place" }))
            .await
            .json();
        assert_eq!(in_place["status"], "in_place");
        assert_eq!(in_place["decision_note"], "Corner desk from Monday");

        // Each decision notified the employee — category + state only,
        // never the words.
        let bells: Value = request
            .get(&format!("/api/employees/{employee}/notifications"))
            .await
            .json();
        let updates: Vec<&Value> = bells
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["kind"] == "adjustment_update")
            .collect();
        assert_eq!(updates.len(), 2, "agreed + in_place");
        let bell_raw = serde_json::to_string(&bells).unwrap();
        assert!(
            !bell_raw.contains("Open-plan noise"),
            "no words in the bell"
        );

        // Save a copy: the subject-access export carries the request
        // verbatim.
        let export: Value = request
            .get(&format!("/api/employees/{employee}/subject-access"))
            .await
            .json();
        assert!(
            serde_json::to_string(&export["adjustment_requests"])
                .unwrap()
                .contains("Open-plan noise"),
            "in writing, on the record"
        );
    })
    .await;
}
