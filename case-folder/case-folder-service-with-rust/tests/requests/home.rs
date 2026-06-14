use case_folder_service_with_rust::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

/// Pins: `/healthz` returns `200` with `status: "ok"`.
#[tokio::test]
#[serial]
async fn healthz_returns_ok() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request.get("/healthz").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "ok");
    })
    .await;
}

/// Pins: with no seeded data, `/api/stats` reports zero across every
/// counter (patients, folder buckets, place kinds, 24h moves).
#[tokio::test]
#[serial]
async fn stats_reports_zeros_on_empty_world() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let response = request.get("/api/stats").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["patients"], 0);
        assert_eq!(body["folders"]["total"], 0);
        assert_eq!(body["folders"]["in_cabinet"], 0);
        assert_eq!(body["folders"]["in_transit"], 0);
        assert_eq!(body["places"]["buildings"], 0);
        assert_eq!(body["places"]["rooms"], 0);
        assert_eq!(body["places"]["cabinets"], 0);
        assert_eq!(body["moves_24h"], 0);
    })
    .await;
}

/// Pins: after seeding one patient, one in-cabinet folder, and a
/// building/room/cabinet chain, `/api/stats` counts each correctly.
#[tokio::test]
#[serial]
async fn stats_counts_folders_and_places() {
    request::<App, _, _>(|request, ctx| async move {
        super::clean_db(&ctx).await;
        let cabinet = super::seed_location_chain("Cabinet S1");
        let alice = super::seed_patient_in_main_patient_service("943 476 5919", "Alice Johnson");
        super::seed_folder(&alice, "Volume 1", Some(&cabinet));

        let response = request.get("/api/stats").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["patients"], 1);
        assert_eq!(body["folders"]["total"], 1);
        assert_eq!(body["folders"]["in_cabinet"], 1);
        assert_eq!(body["places"]["buildings"], 1);
        assert_eq!(body["places"]["rooms"], 1);
        assert_eq!(body["places"]["cabinets"], 1);
    })
    .await;
}
