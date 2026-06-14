use case_folder_service_with_rust::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Pins: `GET /api/folders?nhs_number=…` returns every folder whose
/// snapshot matches that NHS number.
#[tokio::test]
#[serial]
async fn folder_lookup_by_nhs_returns_matching_folders() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet M1");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        super::seed_folder(&alice, "Volume 1", Some(&cabinet));
        super::seed_folder(&alice, "Maternity", Some(&cabinet));

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
        assert!(titles.contains(&"Volume 1"));
        assert!(titles.contains(&"Maternity"));
    })
    .await;
}

/// Pins: a folder lookup for an unseeded NHS number returns `200` with
/// an empty `items` list (not a 404).
#[tokio::test]
#[serial]
async fn folder_lookup_by_unknown_nhs_returns_empty_list() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .get("/api/folders?nhs_number=987%20654%203210")
            .await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["items"].as_array().unwrap().len(), 0);
    })
    .await;
}

/// Pins: `GET /api/workers` proxies the Main Worker Service and returns
/// every seeded worker.
#[tokio::test]
#[serial]
async fn workers_list_returns_workers_from_main_worker_service() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        super::seed_worker_in_main_worker_service("Alice Johnson", Some("nurse"));
        super::seed_worker_in_main_worker_service("Dr Bob Carter", Some("doctor"));

        let response = request.get("/api/workers").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        let names: Vec<&str> = body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"Alice Johnson"));
        assert!(names.contains(&"Dr Bob Carter"));
    })
    .await;
}

/// Pins: a move with a `worker_id` snapshots the worker's name+role
/// (overriding the free-text `moved_by`), returns `201` with a
/// `Location` header, records the event, and relocates the folder.
#[tokio::test]
#[serial]
async fn create_move_records_worker_snapshot() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let from = super::seed_location_chain("Cabinet M2-from");
        let to = super::seed_location_chain("Cabinet M2-to");
        let bob = super::seed_patient_in_main_patient_service("987 654 3210", "Bob Smith");
        let folder = super::seed_folder(&bob, "Cardiology", Some(&from));
        let nurse = super::seed_worker_in_main_worker_service("Alice Johnson", Some("nurse"));

        let response = request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": folder.id.to_string(),
                "to_cabinet_id": to.id.to_string(),
                "worker_id": nurse.id.to_string(),
                "moved_by": "(ignored when worker_id is set)",
                "reason": "Integration test",
            }))
            .await;
        assert_eq!(response.status_code(), 201);
        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(location.starts_with("/api/moves/"));

        let body: serde_json::Value = response.json();
        assert_eq!(body["moved_by"], "Alice Johnson");
        assert_eq!(body["worker_role"], "nurse");
        assert!(
            body["to_cabinet_label"]
                .as_str()
                .unwrap()
                .contains("Cabinet M2-to")
        );

        let events = super::main_event_service().snapshot();
        let event = events
            .iter()
            .find(|e| e.folder_id == folder.id)
            .expect("event");
        assert_eq!(event.worker_id, Some(nurse.id));
        assert_eq!(event.moved_by, "Alice Johnson");
        assert_eq!(event.worker_role_snapshot.as_deref(), Some("nurse"));

        let updated = super::main_thing_service()
            .snapshot()
            .into_iter()
            .find(|f| f.id == folder.id)
            .expect("folder");
        assert_eq!(updated.cabinet_id, Some(to.id));
    })
    .await;
}

/// Pins: a move with no `worker_id` records the free-text `moved_by`
/// verbatim and leaves the worker id / role snapshot empty.
#[tokio::test]
#[serial]
async fn create_move_falls_back_to_free_text_when_no_worker_id() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let from = super::seed_location_chain("Cabinet M3-from");
        let to = super::seed_location_chain("Cabinet M3-to");
        let carol = super::seed_patient_in_main_patient_service("999 999 9999", "Carol Williams");
        let folder = super::seed_folder(&carol, "General", Some(&from));

        let response = request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": folder.id.to_string(),
                "to_cabinet_id": to.id.to_string(),
                "moved_by": "Mira (records)",
            }))
            .await;
        assert_eq!(response.status_code(), 201);

        let events = super::main_event_service().snapshot();
        let event = events
            .iter()
            .find(|e| e.folder_id == folder.id)
            .expect("event");
        assert_eq!(event.worker_id, None);
        assert_eq!(event.moved_by, "Mira (records)");
        assert_eq!(event.worker_role_snapshot, None);
    })
    .await;
}

/// Pins: a malformed (non-UUID) `folder_id` is rejected `422` with a
/// per-field `errors.folder_id` message.
#[tokio::test]
#[serial]
async fn create_move_with_invalid_folder_id_returns_422() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": "not-a-uuid",
                "to_cabinet_id": null,
            }))
            .await;
        assert_eq!(response.status_code(), 422);
        let body: serde_json::Value = response.json();
        assert!(body["errors"]["folder_id"].is_string());
    })
    .await;
}

/// Pins: a well-formed but unknown `folder_id` returns `404`.
#[tokio::test]
#[serial]
async fn create_move_with_unknown_folder_returns_404() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": "00000000-0000-0000-0000-000000000000",
                "to_cabinet_id": null,
            }))
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}

/// Pins: `GET /api/moves` returns the move-event audit log (one entry
/// per recorded move).
#[tokio::test]
#[serial]
async fn moves_list_returns_audit_log() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let from = super::seed_location_chain("Cabinet M4-from");
        let to = super::seed_location_chain("Cabinet M4-to");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        let folder = super::seed_folder(&alice, "Volume 1", Some(&from));

        request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": folder.id.to_string(),
                "to_cabinet_id": to.id.to_string(),
                "moved_by": "Joe Porter",
            }))
            .await;

        let response = request.get("/api/moves").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        let items = body["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["moved_by"], "Joe Porter");
    })
    .await;
}

/// Pins: `GET /api/moves/{id}` returns the single move event by id with
/// its folder title and mover.
#[tokio::test]
#[serial]
async fn show_returns_single_move() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let from = super::seed_location_chain("Cabinet M5-from");
        let to = super::seed_location_chain("Cabinet M5-to");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        let folder = super::seed_folder(&alice, "Volume 1", Some(&from));

        let created = request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": folder.id.to_string(),
                "to_cabinet_id": to.id.to_string(),
                "moved_by": "Joe Porter",
            }))
            .await;
        assert_eq!(created.status_code(), 201);
        let created_body: serde_json::Value = created.json();
        let id = created_body["id"].as_str().unwrap().to_string();

        let response = request.get(&format!("/api/moves/{id}")).await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["id"], id);
        assert_eq!(body["folder_title"], "Volume 1");
        assert_eq!(body["moved_by"], "Joe Porter");
    })
    .await;
}

/// Pins: `GET /api/moves/{id}` for an unknown id returns `404`.
#[tokio::test]
#[serial]
async fn show_unknown_move_returns_404() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .get("/api/moves/00000000-0000-0000-0000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}
