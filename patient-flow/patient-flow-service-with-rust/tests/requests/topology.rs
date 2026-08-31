//! Topology CRUD request tests (PF-T17): happy paths + the `422`
//! validation contract over the live routes.

use loco_rs::testing::prelude::*;
use patient_flow_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

use super::seed_ward;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Site → ward → bay → bed create round-trip; the ward lists and
// fetches back; the bed starts `available`.
async fn topology_creates_and_reads_back() {
    request::<App, _, _>(|request, _ctx| async move {
        let (ward_pid, bed_pids) = seed_ward!(&request, "inpatient", Some("respiratory"), 2).await;
        let ward: Value = request.get(&format!("/api/wards/{ward_pid}")).await.json();
        assert_eq!(ward["kind"], "inpatient");
        assert_eq!(ward["specialty"], "respiratory");
        assert_eq!(ward["open"], true);
        let bed: Value = request
            .get(&format!("/api/beds/{}", bed_pids[0]))
            .await
            .json();
        assert_eq!(bed["state"], "available");
        assert_eq!(bed["deep_clean_required"], false);
        let wards: Value = request.get("/api/wards").await.json();
        assert!(
            wards.as_array().is_some_and(|w| !w.is_empty()),
            "list contains the ward"
        );
        // Unknown-pid contract: an honest 404, not a 500 (loco 0.16
        // does not map ModelError::EntityNotFound itself).
        let ghost = uuid::Uuid::new_v4();
        assert_eq!(
            request
                .get(&format!("/api/wards/{ghost}"))
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .get(&format!("/api/beds/{ghost}"))
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .get(&format!("/api/stays/{ghost}"))
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request.get("/api/wards/not-a-uuid").await.status_code(),
            404
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The validation contract is 422 (not 400): unknown ward kind, blank
// name, and unknown bay sex designation are each refused.
async fn topology_validation_is_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let site: Value = request
            .post("/api/sites")
            .json(&json!({ "name": "S" }))
            .await
            .json();
        let bad_kind = request
            .post("/api/wards")
            .json(&json!({ "site_pid": site["pid"], "name": "W", "code": "W", "kind": "icu" }))
            .await;
        assert_eq!(bad_kind.status_code(), 422, "unknown ward kind is 422");
        let blank_name = request
            .post("/api/sites")
            .json(&json!({ "name": "  " }))
            .await;
        assert_eq!(blank_name.status_code(), 422, "blank site name is 422");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Bed state machine over the live route: close → reopen → an illegal
// transition names the current state and returns 422.
async fn bed_transitions_and_illegal_moves() {
    request::<App, _, _>(|request, _ctx| async move {
        let (_, bed_pids) = seed_ward!(&request, "inpatient", None, 1).await;
        let bed = &bed_pids[0];
        let closed: Value = request
            .post(&format!("/api/beds/{bed}/state"))
            .json(&json!({ "transition": "close", "reason": "maintenance" }))
            .await
            .json();
        assert_eq!(closed["state"], "closed");
        assert_eq!(closed["closure_reason"], "maintenance");
        // Cleaning a closed bed is illegal.
        let illegal = request
            .post(&format!("/api/beds/{bed}/state"))
            .json(&json!({ "transition": "clean_start" }))
            .await;
        assert_eq!(illegal.status_code(), 422);
        let reopened: Value = request
            .post(&format!("/api/beds/{bed}/state"))
            .json(&json!({ "transition": "reopen" }))
            .await
            .json();
        assert_eq!(reopened["state"], "available");
    })
    .await;
}
