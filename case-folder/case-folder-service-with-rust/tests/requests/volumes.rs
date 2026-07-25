use case_folder_service_with_rust::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Pins: `POST /api/volumes` creates an empty volume (`201` + `Location`,
/// status `in-cabinet`, 0 folders) and `GET /api/volumes/{id}` shows it
/// with empty folders and history.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn create_and_show_volume() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cab = super::seed_location_chain("Cabinet V1");
        super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");

        let response = request
            .post("/api/volumes")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5919",
                "title": "Alice Johnson — Vol 1",
                "cabinet_id": cab.id.to_string(),
            }))
            .await;
        assert_eq!(response.status_code(), 201);
        let location = response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap();
        assert!(location.starts_with("/api/volumes/"));
        let body: serde_json::Value = response.json();
        assert_eq!(body["title"], "Alice Johnson — Vol 1");
        assert_eq!(body["status"], "in-cabinet");
        assert_eq!(body["folder_count"], 0);

        let id = body["id"].as_str().unwrap();
        let show = request.get(&format!("/api/volumes/{id}")).await;
        assert_eq!(show.status_code(), 200);
        let sbody: serde_json::Value = show.json();
        assert_eq!(sbody["folders"].as_array().unwrap().len(), 0);
        assert_eq!(sbody["history"].as_array().unwrap().len(), 0);
    })
    .await;
}

/// Pins: creating a volume for an NHS number with no patient record is
/// rejected `422` with a per-field `errors.nhs_number` message.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn create_volume_for_unknown_patient_returns_422() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .post("/api/volumes")
            .json(&serde_json::json!({ "nhs_number": "943 476 5919", "title": "Orphan" }))
            .await;
        assert_eq!(response.status_code(), 422);
        let body: serde_json::Value = response.json();
        assert!(body["errors"]["nhs_number"].is_string());
    })
    .await;
}

/// Pins volume membership: assigning a same-patient folder succeeds,
/// assigning a different patient's folder is rejected `422`, and
/// removing a folder empties the volume again.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn assign_and_remove_folder() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cab = super::seed_location_chain("Cabinet V2");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        let folder = super::seed_folder(&alice, "General", Some(&cab));
        let created = request
            .post("/api/volumes")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5919", "title": "Alice — Vol 1", "cabinet_id": cab.id.to_string(),
            }))
            .await;
        assert_eq!(created.status_code(), 201);
        let id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        // Assign.
        let assigned = request
            .post(&format!("/api/volumes/{id}/folders"))
            .json(&serde_json::json!({ "folder_id": folder.id.to_string() }))
            .await;
        assert_eq!(assigned.status_code(), 200);
        let abody: serde_json::Value = assigned.json();
        let titles: Vec<&str> = abody["folders"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["title"].as_str().unwrap())
            .collect();
        assert_eq!(titles, vec!["General"]);
        assert_eq!(abody["folder_count"], 1);

        // A different patient's folder is rejected.
        let bob = super::seed_patient_in_main_patient_service("987 654 3210", "Bob Smith");
        let bob_folder = super::seed_folder(&bob, "General", Some(&cab));
        let rejected = request
            .post(&format!("/api/volumes/{id}/folders"))
            .json(&serde_json::json!({ "folder_id": bob_folder.id.to_string() }))
            .await;
        assert_eq!(rejected.status_code(), 422);

        // Remove.
        let removed = request
            .delete(&format!("/api/volumes/{id}/folders/{}", folder.id))
            .await;
        assert_eq!(removed.status_code(), 200);
        let rbody: serde_json::Value = removed.json();
        assert_eq!(rbody["folders"].as_array().unwrap().len(), 0);
    })
    .await;
}

/// Pins: `PATCH /api/volumes/{id}` renames the volume's title.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn rename_volume() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cab = super::seed_location_chain("Cabinet V3");
        super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        let created = request
            .post("/api/volumes")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5919", "title": "Original", "cabinet_id": cab.id.to_string(),
            }))
            .await;
        assert_eq!(created.status_code(), 201);
        let id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let renamed = request
            .patch(&format!("/api/volumes/{id}"))
            .json(&serde_json::json!({ "title": "Renamed Vol" }))
            .await;
        assert_eq!(renamed.status_code(), 200);
        let body: serde_json::Value = renamed.json();
        assert_eq!(body["title"], "Renamed Vol");
    })
    .await;
}

/// Pins: `POST /api/volumes/{id}/move` relocates every member folder to
/// the target cabinet (verified in the Thing stub) and records one move
/// event per member folder in the Event stub.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn move_volume_relocates_every_member() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let from = super::seed_location_chain("Cabinet VM-from");
        let to = super::seed_location_chain("Cabinet VM-to");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        let f1 = super::seed_folder(&alice, "General", Some(&from));
        let f2 = super::seed_folder(&alice, "Maternity", Some(&from));
        let created = request
            .post("/api/volumes")
            .json(&serde_json::json!({
                "nhs_number": "943 476 5919", "title": "Alice — Vol 1", "cabinet_id": from.id.to_string(),
            }))
            .await;
        assert_eq!(created.status_code(), 201);
        let id = created.json::<serde_json::Value>()["id"]
            .as_str()
            .unwrap()
            .to_string();

        for f in [&f1, &f2] {
            let r = request
                .post(&format!("/api/volumes/{id}/folders"))
                .json(&serde_json::json!({ "folder_id": f.id.to_string() }))
                .await;
            assert_eq!(r.status_code(), 200);
        }

        let moved = request
            .post(&format!("/api/volumes/{id}/move"))
            .json(&serde_json::json!({
                "to_cabinet_id": to.id.to_string(),
                "moved_by": "Joe Porter",
            }))
            .await;
        assert_eq!(moved.status_code(), 200);
        let body: serde_json::Value = moved.json();
        // Every member folder now reports the destination cabinet.
        for folder in body["folders"].as_array().unwrap() {
            assert_eq!(folder["status"], "in-cabinet");
            assert!(folder["cabinet_label"]
                .as_str()
                .unwrap()
                .contains("Cabinet VM-to"));
        }

        // Both folders were physically relocated in the Thing service.
        let folders = super::main_thing_service().snapshot();
        for fid in [f1.id, f2.id] {
            let f = folders.iter().find(|f| f.id == fid).unwrap();
            assert_eq!(f.cabinet_id, Some(to.id));
        }

        // One move event recorded per member folder.
        let events = super::main_event_service().snapshot();
        assert!(events.iter().any(|e| e.folder_id == f1.id && e.to_cabinet_id == Some(to.id)));
        assert!(events.iter().any(|e| e.folder_id == f2.id && e.to_cabinet_id == Some(to.id)));
    })
    .await;
}

/// Pins: `GET /api/volumes/{id}` for an unknown id returns `404`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn show_unknown_volume_returns_404() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request
            .get("/api/volumes/00000000-0000-0000-0000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}
