//! Request-level tests for match evidence
//! (`agents/share/match-evidence.md`): evidence on every match result,
//! the `Proven`-only auto-merge gate, and evidence persisted with
//! review-queue rows. `#[ignore]`d on PostgreSQL like the rest of the
//! suite.

use axa_test::prelude::*;
use organization_service::app::App;
use serde_json::json;
use serial_test::serial;

use super::seed_directly;

const LEI_A: &str = "5493001KJTIIGC8Y1R12";
const LEI_B: &str = "529900T8BM49AURSDO55";

/// Run `deduplicate` with an auto-merge threshold and return the report.
async fn scan(request: &axa_test::TestServer) -> serde_json::Value {
    let response = request
        .post("/api/organizations/deduplicate")
        .json(&json!({ "auto_merge_threshold": 0.9 }))
        .await;
    assert_eq!(response.status_code(), 200, "scan should succeed");
    response.json()
}

/// Two records agreeing only on a name score above the auto-merge
/// threshold, yet the pair stays `pending` with `consistent` evidence,
/// stored and returned by the list endpoint.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn a_name_only_pair_stays_pending_as_consistent() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        for _ in 0..2 {
            seed_directly(&ctx, json!({ "name": "Zephyrine Holdings" })).await;
        }
        let report = scan(&request).await;
        assert_eq!(report["duplicates_found"], 1);
        assert_eq!(report["auto_merged"], 0);
        let item = &report["review_items"][0];
        assert!(item["match_score"].as_f64().unwrap() >= 0.9, "{item}");
        assert_eq!(item["status"], "pending");
        assert_eq!(item["evidence"]["strength"], "consistent");
        assert!(item["evidence"]["components"].is_array());

        let listed: serde_json::Value = request.get("/api/organizations/review-queue").await.json();
        assert_eq!(listed["items"][0]["evidence"], item["evidence"]);
    })
    .await;
}

/// A shared LEI is proof: the pair is marked `automerged` with
/// `proven` evidence.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn a_shared_identifier_is_automerged_as_proven() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        for name in ["Acme, Inc.", "Acme Inc"] {
            seed_directly(
                &ctx,
                json!({ "name": name, "identifiers": [{ "scheme": "Lei", "value": LEI_A }] }),
            )
            .await;
        }
        let report = scan(&request).await;
        assert_eq!(report["auto_merged"], 1);
        assert_eq!(report["queued_for_review"], 0);
        let item = &report["review_items"][0];
        assert_eq!(item["status"], "automerged");
        assert_eq!(item["evidence"]["strength"], "proven");

        let listed: serde_json::Value = request
            .get("/api/organizations/review-queue?status=automerged")
            .await
            .json();
        assert_eq!(listed["items"][0]["evidence"]["strength"], "proven");
    })
    .await;
}

/// Conflicting LEIs contradict: the pair stays `pending` with
/// `contradicted` evidence naming the scheme.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn conflicting_identifiers_stay_pending_as_contradicted() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        for lei in [LEI_A, LEI_B] {
            seed_directly(
                &ctx,
                json!({
                    "name": "Zephyrine Holdings",
                    "identifiers": [{ "scheme": "Lei", "value": lei }]
                }),
            )
            .await;
        }
        let report = scan(&request).await;
        assert_eq!(report["auto_merged"], 0);
        let item = &report["review_items"][0];
        assert_eq!(item["status"], "pending");
        assert_eq!(item["evidence"]["strength"], "contradicted");
        assert_eq!(
            item["evidence"]["contradictions"][0]["field"],
            "identifier:lei"
        );
    })
    .await;
}

/// `check-duplicates` and the create-time `409` both carry evidence.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn check_duplicates_and_the_409_carry_evidence() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let lei = json!([{ "scheme": "Lei", "value": LEI_A }]);
        let created = request
            .post("/api/organizations")
            .json(&json!({ "name": "Acme, Inc.", "identifiers": lei }))
            .await;
        assert_eq!(created.status_code(), 200);

        let hits: serde_json::Value = request
            .post("/api/organizations/check-duplicates")
            .json(&json!({ "name": "Acme Inc", "identifiers": lei }))
            .await
            .json();
        assert_eq!(hits[0]["evidence"]["strength"], "proven", "{hits}");

        let conflict = request
            .post("/api/organizations")
            .json(&json!({ "name": "Acme Inc", "identifiers": lei }))
            .await;
        assert_eq!(conflict.status_code(), 409);
        let body: serde_json::Value = conflict.json();
        assert_eq!(
            body["errors"][0]["evidence"]["strength"], "proven",
            "{body}"
        );
    })
    .await;
}

/// A review row written without evidence (as every row queued before the
/// column existed was) lists with `evidence: null`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn a_row_without_evidence_lists_as_null() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        use organization_service::models::review_queue::{NewReviewItem, upsert};
        upsert(
            &ctx.db,
            &[NewReviewItem {
                record_id_a: uuid::Uuid::new_v4(),
                record_id_b: uuid::Uuid::new_v4(),
                match_score: 0.9,
                match_quality: "medium".into(),
                detection_method: "batch_deduplication".into(),
                score_breakdown: None,
                status: "pending".into(),
                provenance: "operator".into(),
                evidence: None,
            }],
        )
        .await
        .expect("upsert");
        let listed: serde_json::Value = request.get("/api/organizations/review-queue").await.json();
        assert_eq!(listed["total"], 1);
        assert!(listed["items"][0]["evidence"].is_null());
    })
    .await;
}
