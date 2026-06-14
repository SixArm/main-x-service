use case_folder_service_with_rust::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn list_filters_by_query() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet F1");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        let bob = super::seed_patient_in_main_patient_service("987 654 3210", "Bob Smith");
        super::seed_folder(&alice, "Volume 1", Some(&cabinet));
        super::seed_folder(&alice, "Maternity", Some(&cabinet));
        super::seed_folder(&bob, "General", Some(&cabinet));

        let full = request.get("/api/folders").await;
        assert_eq!(full.status_code(), 200);
        let body: serde_json::Value = full.json();
        let titles: Vec<&str> = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles.len(), 3);
        assert!(titles.contains(&"Volume 1"));
        assert!(titles.contains(&"Maternity"));
        assert!(titles.contains(&"General"));

        let filtered = request.get("/api/folders?q=Maternity").await;
        assert_eq!(filtered.status_code(), 200);
        let body: serde_json::Value = filtered.json();
        let titles: Vec<&str> = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Maternity"]);
        assert_eq!(body["query"], "Maternity");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn list_filters_by_nhs_number() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet F1b");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        let bob = super::seed_patient_in_main_patient_service("987 654 3210", "Bob Smith");
        super::seed_folder(&alice, "Volume 1", Some(&cabinet));
        super::seed_folder(&bob, "General", Some(&cabinet));

        let response = request
            .get("/api/folders?nhs_number=943%20476%205919")
            .await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        let titles: Vec<&str> = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["Volume 1"]);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_folder_creates_patient_via_main_patient_service_and_returns_201() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet F2");

        let response = request
            .post("/api/folders")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5919",
                "patient_name": "Alice Johnson",
                "date_of_birth": "1991-04-12",
                "title": "Volume 1",
                "cabinet_id": cabinet.id.to_string(),
                "notes": null,
            }))
            .await;
        assert_eq!(response.status_code(), 201);
        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(location.starts_with("/api/folders/"));

        let body: serde_json::Value = response.json();
        assert_eq!(body["title"], "Volume 1");
        assert_eq!(body["status"], "in-cabinet");

        let state = super::main_patient_service().snapshot();
        assert!(state.iter().any(|p| p.name == "Alice Johnson"));
        let folders = super::main_thing_service().snapshot();
        assert!(folders.iter().any(|f| f.title == "Volume 1"));
        let events = super::main_event_service().snapshot();
        assert!(events.iter().any(|e| e.folder_title == "Volume 1"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_folder_attaches_to_existing_main_patient_service_patient() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet F3");
        super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");

        let response = request
            .post("/api/folders")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5919",
                "title": "Cardiology 2023",
                "cabinet_id": cabinet.id.to_string(),
            }))
            .await;
        assert_eq!(response.status_code(), 201);
        let body: serde_json::Value = response.json();
        assert_eq!(body["title"], "Cardiology 2023");
        assert_eq!(body["patient_name"], "Alice Johnson");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_folder_with_invalid_nhs_returns_422() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/folders")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5918",
                "patient_name": "Invalid Patient",
                "date_of_birth": "1990-01-01",
                "title": "General",
            }))
            .await;
        assert_eq!(response.status_code(), 422);
        let body: serde_json::Value = response.json();
        assert!(
            body["errors"]["nhs_number"]
                .as_str()
                .unwrap()
                .contains("Modulus 11")
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn show_unknown_folder_returns_404() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .get("/api/folders/00000000-0000-0000-0000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404);
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"], "Folder not found");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn folder_history_lists_events_for_folder() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet F4");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        let folder = super::seed_folder(&alice, "Volume 1", Some(&cabinet));

        // Empty history is still a 200 with an empty items array.
        let response = request
            .get(&format!("/api/folders/{}/history", folder.id))
            .await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["items"].as_array().unwrap().len(), 0);
    })
    .await;
}
