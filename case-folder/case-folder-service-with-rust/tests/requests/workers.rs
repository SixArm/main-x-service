use case_folder_service_with_rust::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn list_returns_workers_from_stub_service() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        super::seed_worker_in_main_worker_service("Alice Johnson", Some("nurse"));
        super::seed_worker_in_main_worker_service("Dr Bob Carter", Some("doctor"));

        let response = request.get("/api/workers").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        let items = body["items"].as_array().unwrap();
        let names: Vec<&str> = items.iter().map(|w| w["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"Alice Johnson"));
        assert!(names.contains(&"Dr Bob Carter"));
        let alice = items.iter().find(|w| w["name"] == "Alice Johnson").unwrap();
        assert_eq!(alice["role"], "nurse");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn list_filters_by_query() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        super::seed_worker_in_main_worker_service("Alice Johnson", Some("nurse"));
        super::seed_worker_in_main_worker_service("Dr Bob Carter", Some("doctor"));

        let response = request.get("/api/workers?q=Bob").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        let items = body["items"].as_array().unwrap();
        let names: Vec<&str> = items.iter().map(|w| w["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["Dr Bob Carter"]);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn show_returns_moved_and_patient_folders() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let from = super::seed_location_chain("Cabinet W-from");
        let to = super::seed_location_chain("Cabinet W-to");
        let carol = super::seed_patient_in_main_patient_service("999 999 9999", "Carol Williams");
        let general = super::seed_folder(&carol, "General", Some(&from));
        // A second folder for the same patient, never moved by Mira.
        super::seed_folder(&carol, "Maternity", Some(&from));
        let mira =
            super::seed_worker_in_main_worker_service("Mira (records)", Some("administrator"));

        // Mira moves only the General folder.
        let mv = request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": general.id.to_string(),
                "to_cabinet_id": to.id.to_string(),
                "worker_id": mira.id.to_string(),
                "reason": "Outpatient appointment",
            }))
            .await;
        assert_eq!(mv.status_code(), 201);

        let response = request.get(&format!("/api/workers/{}", mira.id)).await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["worker"]["name"], "Mira (records)");

        // moved_folders = just what Mira moved.
        let moved: Vec<&str> = body["moved_folders"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap())
            .collect();
        assert_eq!(moved, vec!["General"]);

        // patient_folders = ALL of that patient's folders (superset).
        let patient_folders: Vec<&str> = body["patient_folders"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap())
            .collect();
        assert!(patient_folders.contains(&"General"));
        assert!(patient_folders.contains(&"Maternity"));

        assert_eq!(body["moves"].as_array().unwrap().len(), 1);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn show_unknown_worker_returns_404() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .get("/api/workers/00000000-0000-0000-0000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}
