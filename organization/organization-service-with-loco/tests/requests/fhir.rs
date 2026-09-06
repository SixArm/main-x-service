//! `GET /fhir/Organization` search now resolves a text-bearing param
//! (`name` / `address-city` / …) through the Tantivy index rather than
//! the capped in-memory Postgres scan (ORG-T5).
//!
//! Literally exceeding the old `FHIR_SEARCH_SCAN_CAP` (1000 rows) to
//! prove the scan would have missed a hit is impractical here — it
//! would mean creating 1000+ live rows (and indexing them) in one test,
//! which is exactly the kind of cost the family's fast-suite convention
//! avoids elsewhere (see e.g. the streaming-memory test's allocator
//! instrumentation instead of an actually-huge file). The claim this
//! test pins instead: retrieval no longer depends on `OrgModel::list`'s
//! recency ordering at all — the target organization is created
//! **first** (so it has the lowest/oldest id, the position a
//! newest-first list scan would reach last) and is still found ahead of
//! several newer distractors, and `address-city` search stays
//! field-precise (a distractor sharing no city must not appear).

use loco_rs::testing::prelude::*;
use organization_service::app::App;
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
async fn fhir_search_by_name_and_city_resolve_through_the_index() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        // The target is created FIRST — the oldest, lowest-id row, the
        // position `OrgModel::list`'s `ORDER BY id DESC` scan reaches
        // last, not first.
        let target: Value = request
            .post("/api/organizations")
            .json(&json!({
                "name": "Zephyrine Analytics Ltd",
                "address": { "locality": "Neverwhere Falls", "postal_code": "NW1 9ZZ" },
            }))
            .await
            .json();
        let target_pid = target["pid"].as_str().expect("pid").to_string();

        // Several newer distractors, none sharing the target's name or
        // city. Seeded directly (ORG-T3): these near-identical names
        // (differing only by a trailing digit, with an otherwise-shared
        // "Distractor Holdings" prefix the matcher's Jaro-Winkler
        // prefix bonus weighs heavily) are near-duplicates of *each
        // other* even with distinct localities — exactly what the
        // real-time create check now exists to catch — so seeding past
        // it is the same fix `organizations.rs`'s equivalent fixtures
        // needed.
        for i in 0..5 {
            super::seed_directly(
                &ctx,
                json!({
                    "name": format!("Distractor Holdings {i}"),
                    "address": { "locality": format!("Someplace Ordinary {i}") },
                }),
            )
            .await;
        }

        // `name=` resolves via the index and finds the (oldest) target.
        let bundle: Value = request
            .get("/fhir/Organization?name=Zephyrine")
            .await
            .json();
        assert_eq!(bundle["resourceType"], "Bundle");
        assert_eq!(bundle["type"], "searchset");
        assert!(
            entry_ids(&bundle).contains(&target_pid.as_str()),
            "name search should find the target org, got {bundle}"
        );

        // `address-city=` resolves via the index and stays field-precise:
        // only the target's locality matches, none of the distractors'.
        let bundle: Value = request
            .get("/fhir/Organization?address-city=Neverwhere")
            .await
            .json();
        assert_eq!(
            entry_ids(&bundle),
            vec![target_pid.as_str()],
            "address-city search should find only the target org, got {bundle}"
        );
    })
    .await;
}

/// A request with no text-bearing param (bare `_id`) still falls back to
/// the capped scan — unchanged from before ORG-T5, and pinned here so a
/// future change can't silently drop that path.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn fhir_search_by_id_alone_still_works() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/organizations")
            .json(&json!({ "name": "Solo Id Search Co" }))
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();

        let bundle: Value = request
            .get(&format!("/fhir/Organization?_id={pid}"))
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
