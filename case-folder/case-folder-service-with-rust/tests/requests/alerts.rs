use case_folder_service_with_rust::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Pins: moving a folder across a building boundary (Hospital A →
/// Hospital B) surfaces a geofence alert naming both buildings.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn cross_building_move_raises_geofence_alert() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let b1 = super::seed_building("Hospital A");
        let r1 = super::seed_room(b1.id, "Room A1");
        let cab_a = super::seed_cabinet(r1.id, "Cab A");
        let b2 = super::seed_building("Hospital B");
        let r2 = super::seed_room(b2.id, "Room B1");
        let cab_b = super::seed_cabinet(r2.id, "Cab B");
        super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");

        let created = request
            .post("/api/folders")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5919",
                "title": "Volume 1",
                "cabinet_id": cab_a.id.to_string(),
            }))
            .await;
        assert_eq!(created.status_code(), 201);
        let folder_id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Move across the building boundary (Hospital A → Hospital B).
        let moved = request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": folder_id,
                "to_cabinet_id": cab_b.id.to_string(),
                "moved_by": "Joe Porter",
            }))
            .await;
        assert_eq!(moved.status_code(), 201);

        let response = request.get("/api/alerts").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        let items = body["items"].as_array().unwrap();
        let alert = items
            .iter()
            .find(|a| a["folder_id"] == folder_id)
            .expect("geofence alert for the cross-building move");
        assert_eq!(alert["from_building"], "Hospital A");
        assert_eq!(alert["to_building"], "Hospital B");
    })
    .await;
}

/// Pins: a move between two cabinets within the same building raises no
/// geofence alert.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn same_building_move_raises_no_alert() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let b = super::seed_building("Hospital A");
        let r = super::seed_room(b.id, "Room A1");
        let c1 = super::seed_cabinet(r.id, "Cab 1");
        let c2 = super::seed_cabinet(r.id, "Cab 2");
        super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");

        let created = request
            .post("/api/folders")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5919",
                "title": "Volume 1",
                "cabinet_id": c1.id.to_string(),
            }))
            .await;
        let folder_id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": folder_id,
                "to_cabinet_id": c2.id.to_string(),
                "moved_by": "Joe Porter",
            }))
            .await;

        let response = request.get("/api/alerts").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        let items = body["items"].as_array().unwrap();
        assert!(
            !items.iter().any(|a| a["folder_id"] == folder_id),
            "a within-building move must not raise a geofence alert"
        );
    })
    .await;
}
