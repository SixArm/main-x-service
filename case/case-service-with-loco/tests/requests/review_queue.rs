//! `GET /api/cases/review-queue` and `POST
//! /api/cases/review-queue/{id}/decision` (T-8): the stored review queue
//! already carries rows written by the BLK-5 bulk-import pipeline (a
//! keyless duplicate candidate, `provenance = "import"`), but nothing
//! exposed it over the API. There is no batch-scan endpoint yet
//! (`/deduplicate`, T-7), so these tests seed a row directly through the
//! model layer (`crate::models::review_queue::upsert`) — the same
//! storage the bulk pipeline and a future `/deduplicate` would both
//! write through.
//!
//! `#[ignore]`d: needs PostgreSQL; run with `cargo test -- --ignored`.

use case_service::app::App;
use case_service::models::review_queue::{NewReviewItem, upsert};
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

fn case_a() -> Value {
    json!({ "title": "Housing benefit appeal (A)", "agency_id": "dwp", "case_number": "HB-2024-1001" })
}

fn case_b() -> Value {
    json!({ "title": "Housing benefit appeal (B)", "agency_id": "dwp", "case_number": "HB-2024-1002" })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn review_queue_lists_a_seeded_pending_item() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let a: Value = request.post("/api/cases").json(&case_a()).await.json();
        let b: Value = request.post("/api/cases").json(&case_b()).await.json();
        let pid_a: uuid::Uuid = a["pid"].as_str().expect("pid").parse().expect("uuid");
        let pid_b: uuid::Uuid = b["pid"].as_str().expect("pid").parse().expect("uuid");

        let stored = upsert(
            &ctx.db,
            &[NewReviewItem {
                record_id_a: pid_a,
                record_id_b: pid_b,
                match_score: 0.87,
                match_quality: "probable".to_string(),
                detection_method: "batch_deduplication".to_string(),
                score_breakdown: None,
                status: "pending".to_string(),
                provenance: "import".to_string(),
            }],
        )
        .await
        .expect("seed review item");
        let item_id = stored[0].id;

        let body: Value = request.get("/api/cases/review-queue").await.json();
        assert_eq!(body["total"], 1, "{body}");
        let item = &body["items"][0];
        assert_eq!(item["id"], item_id.to_string());
        assert_eq!(item["status"], "pending");
        assert_eq!(item["provenance"], "import");
        // Field order is unordered by insertion (normalized pair), so
        // just check both ids are present, not the exact order.
        let ids: Vec<&str> = [item["case_id_a"].as_str(), item["case_id_b"].as_str()]
            .into_iter()
            .flatten()
            .collect();
        assert!(ids.contains(&pid_a.to_string().as_str()));
        assert!(ids.contains(&pid_b.to_string().as_str()));

        // An unknown status filter is 422.
        let bad = request.get("/api/cases/review-queue?status=bogus").await;
        assert_eq!(bad.status_code(), 422, "unknown status token should be 422");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_decision_is_first_writer_wins() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let a: Value = request.post("/api/cases").json(&case_a()).await.json();
        let b: Value = request.post("/api/cases").json(&case_b()).await.json();
        let pid_a: uuid::Uuid = a["pid"].as_str().expect("pid").parse().expect("uuid");
        let pid_b: uuid::Uuid = b["pid"].as_str().expect("pid").parse().expect("uuid");

        let stored = upsert(
            &ctx.db,
            &[NewReviewItem {
                record_id_a: pid_a,
                record_id_b: pid_b,
                match_score: 0.9,
                match_quality: "probable".to_string(),
                detection_method: "batch_deduplication".to_string(),
                score_breakdown: None,
                status: "pending".to_string(),
                provenance: "import".to_string(),
            }],
        )
        .await
        .expect("seed review item");
        let item_id = stored[0].id;

        // First decision: confirmed.
        let decided: Value = request
            .post(&format!("/api/cases/review-queue/{item_id}/decision"))
            .json(&json!({ "status": "confirmed" }))
            .await
            .json();
        assert_eq!(decided["status"], "confirmed");

        // Second decision on the same item: 422 (already decided).
        let second = request
            .post(&format!("/api/cases/review-queue/{item_id}/decision"))
            .json(&json!({ "status": "rejected" }))
            .await;
        assert_eq!(
            second.status_code(),
            422,
            "a second decision on an already-decided item should be 422"
        );

        // A decision on an unknown id is 404.
        let unknown = request
            .post(&format!(
                "/api/cases/review-queue/{}/decision",
                uuid::Uuid::new_v4()
            ))
            .json(&json!({ "status": "confirmed" }))
            .await;
        assert_eq!(unknown.status_code(), 404);
    })
    .await;
}
