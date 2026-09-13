//! Request-level integration tests for the batch `/deduplicate` scan and
//! the duplicate review queue it (and the bulk-import pipeline) feed
//! (spec `13-tasks.md` CP-T1).
//!
//! These boot the real loco app against the `test` environment config,
//! so they require a reachable PostgreSQL instance and are `#[ignore]`d
//! (family convention; run with `cargo test -- --ignored`).

use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

/// Full round trip: scan finds a likely duplicate, the pair is listed
/// pending, deciding it confirms it, a second decision on the same pair
/// is `422` (first-writer-wins), and deciding an unknown id is `404`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn scan_list_decide_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // A unique name (per test run) so this run's pairwise scan is
        // not muddied by other tests' fixtures sharing the corpus.
        let name = format!(
            "DedupScan {}",
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        );

        let a: Value = request
            .post("/api/care-pathways")
            .json(&json!({"name": name}))
            .await
            .json();
        let b: Value = request
            .post("/api/care-pathways")
            .json(&json!({"name": name}))
            .await
            .json();
        let pid_a = a["pid"].as_str().expect("pid").to_string();
        let pid_b = b["pid"].as_str().expect("pid").to_string();

        // Scan: an identical-name pair is a certain match (score 1.0 via
        // the name component alone, with no other fields to disagree).
        let scan: Value = request
            .post("/api/care-pathways/deduplicate")
            .json(&json!({}))
            .await
            .json();
        assert!(
            scan["pathways_scanned"].as_u64().unwrap() >= 2,
            "scan: {scan:?}"
        );
        let items = scan["review_items"].as_array().expect("review_items");
        let pair = items
            .iter()
            .find(|i| {
                let a_ = i["pathway_id_a"].as_str().unwrap();
                let b_ = i["pathway_id_b"].as_str().unwrap();
                (a_ == pid_a && b_ == pid_b) || (a_ == pid_b && b_ == pid_a)
            })
            .unwrap_or_else(|| panic!("scan must find the seeded pair: {scan:?}"));
        assert_eq!(pair["provenance"], "operator");
        assert_eq!(pair["detection_method"], "batch_deduplication");
        assert_eq!(pair["status"], "pending");
        let item_id = pair["id"].as_str().expect("id").to_string();

        // List: the pair appears under the pending filter.
        let queued: Value = request
            .get("/api/care-pathways/review-queue?status=pending")
            .await
            .json();
        let listed = queued["items"]
            .as_array()
            .expect("items")
            .iter()
            .any(|i| i["id"] == item_id);
        assert!(listed, "queue: {queued:?}");

        // Re-scanning the same corpus upserts the same row rather than
        // duplicating it (normalized-pair upsert, stable id).
        let rescan: Value = request
            .post("/api/care-pathways/deduplicate")
            .json(&json!({}))
            .await
            .json();
        let rescanned_id = rescan["review_items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| {
                let a_ = i["pathway_id_a"].as_str().unwrap();
                let b_ = i["pathway_id_b"].as_str().unwrap();
                (a_ == pid_a && b_ == pid_b) || (a_ == pid_b && b_ == pid_a)
            })
            .expect("re-scan must still find the pair")["id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(rescanned_id, item_id, "re-scan must upsert, not duplicate");

        // Decide: confirm the pending pair.
        let decide_url = format!("/api/care-pathways/review-queue/{item_id}/decision");
        let decided = request
            .post(&decide_url)
            .json(&json!({"status": "confirmed"}))
            .await;
        assert_eq!(decided.status_code(), 200, "body: {:?}", decided.text());
        let decided_json: Value = decided.json();
        assert_eq!(decided_json["status"], "confirmed");

        // Decide again: first-writer-wins, already-decided is 422.
        let second = request
            .post(&decide_url)
            .json(&json!({"status": "rejected"}))
            .await;
        assert_eq!(second.status_code(), 422, "body: {:?}", second.text());

        // Deciding an unknown id is 404.
        let unknown_url = format!(
            "/api/care-pathways/review-queue/{}/decision",
            uuid::Uuid::new_v4()
        );
        let unknown = request
            .post(&unknown_url)
            .json(&json!({"status": "confirmed"}))
            .await;
        assert_eq!(unknown.status_code(), 404);
    })
    .await;
}

/// An unrelated pair (distinct names, nothing else in common) scores
/// below the matcher's threshold and is never queued.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn unrelated_pathways_are_not_queued() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let suffix = &uuid::Uuid::new_v4().simple().to_string()[..8];
        let a: Value = request
            .post("/api/care-pathways")
            .json(&json!({"name": format!("Unrelated Alpha {suffix}")}))
            .await
            .json();
        let b: Value = request
            .post("/api/care-pathways")
            .json(&json!({"name": format!("Totally Different Zeta {suffix}")}))
            .await
            .json();
        let pid_a = a["pid"].as_str().expect("pid").to_string();
        let pid_b = b["pid"].as_str().expect("pid").to_string();

        let scan: Value = request
            .post("/api/care-pathways/deduplicate")
            .json(&json!({}))
            .await
            .json();
        let items = scan["review_items"].as_array().expect("review_items");
        let found = items.iter().any(|i| {
            let a_ = i["pathway_id_a"].as_str().unwrap();
            let b_ = i["pathway_id_b"].as_str().unwrap();
            (a_ == pid_a && b_ == pid_b) || (a_ == pid_b && b_ == pid_a)
        });
        assert!(!found, "an unrelated pair must not be queued: {scan:?}");
    })
    .await;
}
