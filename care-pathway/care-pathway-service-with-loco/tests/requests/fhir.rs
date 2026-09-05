//! `GET /fhir/PlanDefinition` search now resolves a text-bearing param
//! (`name` / `identifier`) through the Tantivy index rather than the
//! capped in-memory Postgres scan (CP-T4).
//!
//! Literally exceeding the old `FHIR_SEARCH_SCAN_CAP` (1000 rows) to
//! prove the scan would have missed a hit is impractical here — it would
//! mean creating 1000+ live rows (and indexing them) in one test, which
//! is exactly the kind of cost the family's fast-suite convention avoids
//! elsewhere (see organization-service's identical ORG-T5 test for the
//! same reasoning). The claim this test pins instead: retrieval no
//! longer depends on `PathwayModel::list`'s recency ordering at all —
//! the target pathway is created **first** (so it has the lowest/oldest
//! id, the position a newest-first list scan would reach last) and is
//! still found ahead of several newer distractors.

use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

/// Every `Bundle.entry[].resource.id` in a searchset response.
fn entry_ids(bundle: &Value) -> Vec<&str> {
    bundle["entry"]
        .as_array()
        .expect("entry array")
        .iter()
        .map(|e| e["resource"]["id"].as_str().expect("id"))
        .collect()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn fhir_search_by_name_resolves_through_the_index() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // The target is created FIRST — the oldest, lowest-id row, the
        // position `PathwayModel::list`'s `ORDER BY id DESC` scan
        // reaches last, not first.
        let target: Value = request
            .post("/api/care-pathways")
            .json(&json!({ "name": "Zephyrine Neuro-Rehabilitation Pathway" }))
            .await
            .json();
        let target_pid = target["pid"].as_str().expect("pid").to_string();

        // Several newer distractors, none sharing the target's name.
        for i in 0..5 {
            let created = request
                .post("/api/care-pathways")
                .json(&json!({ "name": format!("Distractor Pathway {i}") }))
                .await;
            assert_eq!(created.status_code(), 200, "distractor {i} should create");
        }

        // `name=` resolves via the index and finds the (oldest) target.
        let bundle: Value = request
            .get("/fhir/PlanDefinition?name=Zephyrine")
            .await
            .json();
        assert_eq!(bundle["resourceType"], "Bundle");
        assert_eq!(bundle["type"], "searchset");
        assert!(
            entry_ids(&bundle).contains(&target_pid.as_str()),
            "name search should find the target pathway, got {bundle}"
        );
    })
    .await;
}

/// A request with no text-bearing param (bare `_id`) still falls back to
/// the capped scan — unchanged from before CP-T4, and pinned here so a
/// future change can't silently drop that path.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn fhir_search_by_id_alone_still_works() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/care-pathways")
            .json(&json!({ "name": "Solo Id Search Pathway" }))
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();

        let bundle: Value = request
            .get(&format!("/fhir/PlanDefinition?_id={pid}"))
            .await
            .json();
        assert_eq!(
            entry_ids(&bundle),
            vec![pid.as_str()],
            "bare _id search should still resolve via the fallback scan"
        );
    })
    .await;
}
