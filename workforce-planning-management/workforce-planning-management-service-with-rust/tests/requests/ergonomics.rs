//! The ergonomic (DSE) round-trip (WPM-R32 / WPM-D24): the default
//! checklist instantiates, completion is gated on every answer, a
//! completed assessment freezes, and issues surface by department.

use workforce_planning_management_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

use super::{activate, an_org, seed_employee};

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn ergonomics_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let org = an_org();
        let employee = seed_employee(&request, &org, "DSE-1", None).await;
        activate(&request, &employee).await;

        // Blank workstation refused; default checklist instantiates.
        assert_eq!(
            request
                .post(&format!("/api/employees/{employee}/ergonomic-assessments"))
                .json(&json!({ "workstation": "  " }))
                .await
                .status_code(),
            422
        );
        let created: Value = request
            .post(&format!("/api/employees/{employee}/ergonomic-assessments"))
            .json(&json!({ "workstation": "Desk 4.12" }))
            .await
            .json();
        let a_pid = created["pid"].as_str().unwrap().to_string();
        let listed: Value = request
            .get(&format!("/api/employees/{employee}/ergonomic-assessments"))
            .await
            .json();
        let assessment = &listed.as_array().unwrap()[0];
        let items = assessment["items"].as_array().unwrap().clone();
        assert_eq!(items.len(), 8, "default DSE checklist");
        assert_eq!(assessment["status"], "open");

        // Completion is gated until every item is answered.
        assert_eq!(
            request
                .post(&format!("/api/ergonomic-assessments/{a_pid}/complete"))
                .await
                .status_code(),
            422,
            "unanswered items block completion"
        );
        // Answer all items; flag one issue with an equipment note.
        for (index, item) in items.iter().enumerate() {
            let item_pid = item["pid"].as_str().unwrap();
            let body = if index == 1 {
                json!({ "ok": false, "note": "chair height lever broken" })
            } else {
                json!({ "ok": true })
            };
            request
                .put(&format!("/api/ergonomic-items/{item_pid}"))
                .json(&body)
                .await
                .assert_status_ok();
        }
        let completed: Value = request
            .post(&format!("/api/ergonomic-assessments/{a_pid}/complete"))
            .await
            .json();
        assert_eq!(completed["status"], "completed");
        assert!(completed["assessed_on"].is_string());
        // A completed assessment is a record: answers freeze.
        let frozen_item = items[0]["pid"].as_str().unwrap();
        assert_eq!(
            request
                .put(&format!("/api/ergonomic-items/{frozen_item}"))
                .json(&json!({ "ok": false }))
                .await
                .status_code(),
            422,
            "completed assessments freeze"
        );

        // The issues report: one engineering issue, with the note.
        let issues: Value = request.get("/api/ergonomics/issues").await.json();
        assert_eq!(issues["by_department"]["engineering"], 1);
        assert_eq!(issues["issues"][0]["workstation"], "Desk 4.12");
        assert_eq!(issues["issues"][0]["note"], "chair height lever broken");

        // A custom checklist is honoured.
        let custom: Value = request
            .post(&format!("/api/employees/{employee}/ergonomic-assessments"))
            .json(&json!({ "workstation": "home office",
                           "items": ["Laptop stand present", "External keyboard present"] }))
            .await
            .json();
        assert!(custom["pid"].is_string());
        let listed: Value = request
            .get(&format!("/api/employees/{employee}/ergonomic-assessments"))
            .await
            .json();
        let newest = &listed.as_array().unwrap()[0];
        assert_eq!(newest["items"].as_array().unwrap().len(), 2);
    })
    .await;
}
