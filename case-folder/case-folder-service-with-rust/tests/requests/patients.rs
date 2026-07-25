use case_folder_service_with_rust::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Pins: `GET /api/patients` derives the patient list from folder
/// snapshots, reporting source `"Main Patient Service"` and folder count.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn list_returns_patients_from_folder_snapshots() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet P0");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        super::seed_folder(&alice, "Volume 1", Some(&cabinet));

        let response = request.get("/api/patients").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["name"], "Alice Johnson");
        assert_eq!(items[0]["nhs_number"], "943 476 5919");
        assert_eq!(items[0]["source"], "Main Patient Service");
        assert_eq!(items[0]["folder_count"], 1);
    })
    .await;
}

/// Pins: `GET /api/patients/{nhs}` for a known patient sets
/// `patient_service_match: true` and lists all of their folders.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn show_lists_folders_for_main_patient_service_patient() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet P1");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        super::seed_folder(&alice, "Volume 1", Some(&cabinet));
        super::seed_folder(&alice, "Maternity", Some(&cabinet));

        let response = request.get("/api/patients/9434765919").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["patient_service_match"], true);
        assert_eq!(body["patient"]["name"], "Alice Johnson");
        let titles: Vec<&str> = body["folders"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap())
            .collect();
        assert!(titles.contains(&"Volume 1"));
        assert!(titles.contains(&"Maternity"));
        assert_eq!(titles.len(), 2);
    })
    .await;
}

/// Pins: when the Patient service has no record, show falls back to the
/// folder snapshot — `patient_service_match: false`, null `patient`, but
/// the snapshot-only folders still listed.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn show_falls_back_to_snapshot_when_main_patient_service_has_no_record() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet P2");
        let phantom = case_folder_service_with_rust::main_patient_service::Patient {
            id: uuid::Uuid::new_v4(),
            nhs_number: "987 654 3210".into(),
            name: "Bob Snapshot".into(),
            date_of_birth: None,
        };
        super::seed_folder(&phantom, "Snapshot only", Some(&cabinet));

        let response = request.get("/api/patients/9876543210").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["patient_service_match"], false);
        assert!(body["patient"].is_null());
        let titles: Vec<&str> = body["folders"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Snapshot only"]);
    })
    .await;
}
