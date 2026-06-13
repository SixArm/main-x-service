use case_tracking_with_loco::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn index_lists_buildings_rooms_and_cabinets() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let building = super::seed_building("Main Hospital");
        let room = super::seed_room(building.id, "Ward A Records Room");
        super::seed_cabinet(room.id, "Cabinet A1");

        let response = request.get("/api/places").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["buildings"][0]["name"], "Main Hospital");
        assert_eq!(body["rooms"][0]["name"], "Ward A Records Room");
        assert_eq!(body["cabinets"][0]["name"], "Cabinet A1");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn index_kind_filter_hides_other_groups() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let building = super::seed_building("Main Hospital");
        let room = super::seed_room(building.id, "Ward A Records Room");
        super::seed_cabinet(room.id, "Cabinet A1");

        let response = request.get("/api/places?kind=cabinet").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["kind"], "cabinet");
        assert!(body["buildings"].as_array().unwrap().is_empty());
        assert!(body["rooms"].as_array().unwrap().is_empty());
        assert_eq!(body["cabinets"][0]["name"], "Cabinet A1");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn show_returns_place_details_and_folders_inside_cabinet() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let building = super::seed_building("Main Hospital");
        let room = super::seed_room(building.id, "Ward A Records Room");
        let cabinet = super::seed_cabinet(room.id, "Cabinet A1");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        super::seed_folder(&alice, "Volume 1", Some(&cabinet));

        let response = request.get(&format!("/api/places/{}", cabinet.id)).await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["name"], "Cabinet A1");
        assert_eq!(body["place_kind"], "cabinet");
        let titles: Vec<&str> = body["folders"]
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
async fn show_unknown_place_returns_404() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .get("/api/places/00000000-0000-0000-0000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_registers_a_new_place_in_the_main_place_service() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;

        let response = request
            .post("/api/places")
            .json(&serde_json::json!({
                "name": "Brand New Hospital",
                "kind": "building",
            }))
            .await;
        assert_eq!(response.status_code(), 201);
        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(location.starts_with("/api/places/"));

        let body: serde_json::Value = response.json();
        assert_eq!(body["name"], "Brand New Hospital");
        assert_eq!(body["place_kind"], "building");

        let state = super::main_place_service().snapshot();
        assert!(state.iter().any(|p| p.name == "Brand New Hospital"));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn create_with_missing_name_returns_422() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/places")
            .json(&serde_json::json!({
                "name": "",
                "kind": "building",
            }))
            .await;
        assert_eq!(response.status_code(), 422);
        let body: serde_json::Value = response.json();
        assert!(body["errors"]["name"].is_string());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cabinet_history_shows_presence_intervals() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let building = super::seed_building("Main Hospital");
        let room = super::seed_room(building.id, "Ward A Records Room");
        let cab_a = super::seed_cabinet(room.id, "Cabinet A1");
        let cab_b = super::seed_cabinet(room.id, "Cabinet A2");
        super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");

        // Create the folder in A1 — records a "Folder created" enter event.
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

        // While resident in A1: one open interval (left_at == null).
        let resp_a = request
            .get(&format!("/api/places/{}/history", cab_a.id))
            .await;
        assert_eq!(resp_a.status_code(), 200);
        let body_a: serde_json::Value = resp_a.json();
        let pres_a = body_a["presences"].as_array().unwrap();
        assert_eq!(pres_a.len(), 1);
        assert_eq!(pres_a[0]["folder_title"], "Volume 1");
        assert!(pres_a[0]["left_at"].is_null());

        // Move A1 -> A2.
        let mv = request
            .post("/api/moves")
            .json(&serde_json::json!({
                "folder_id": folder_id,
                "to_cabinet_id": cab_b.id.to_string(),
                "moved_by": "Joe Porter",
            }))
            .await;
        assert_eq!(mv.status_code(), 201);

        // A1 now shows a closed interval (left_at set).
        let body_a2: serde_json::Value = request
            .get(&format!("/api/places/{}/history", cab_a.id))
            .await
            .json();
        let pres_a2 = body_a2["presences"].as_array().unwrap();
        assert_eq!(pres_a2.len(), 1);
        assert!(!pres_a2[0]["left_at"].is_null());

        // A2 shows the folder now resident (open).
        let body_b: serde_json::Value = request
            .get(&format!("/api/places/{}/history", cab_b.id))
            .await
            .json();
        let pres_b = body_b["presences"].as_array().unwrap();
        assert_eq!(pres_b.len(), 1);
        assert!(pres_b[0]["left_at"].is_null());

        // The building aggregates both cabinets' histories.
        let body_bldg: serde_json::Value = request
            .get(&format!("/api/places/{}/history", building.id))
            .await
            .json();
        assert!(body_bldg["presences"].as_array().unwrap().len() >= 2);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn history_unknown_place_returns_404() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .get("/api/places/00000000-0000-0000-0000-000000000000/history")
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}
