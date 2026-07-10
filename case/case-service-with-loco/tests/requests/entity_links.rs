//! DB-gated request tests for the cross-service entity-link write side
//! (`agents/share/cross-service-linking.md` §4.1, §4.2) — the
//! `subject_of` (case → person) edge the case service originates.
//!
//! These boot the app against the `test` environment (needs PostgreSQL;
//! `config/test.yaml` / `DATABASE_URL`) and so are `#[ignore]`d. They
//! drive the real HTTP surface: create a case, `POST` a `subject_of`
//! link, `GET` it back, `DELETE` it, and assert that a `linked` then an
//! `unlinked` event surface on `/api/cases/events/recent` (the default
//! `memory` transport). Enforcement is off by default, so no token.
//!
//! The accept/reject validation matrix is pinned DB-free in
//! `src/controllers/links.rs`.

use case_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

/// Create a case and return its `pid`.
async fn create_case(request: &loco_rs::TestServer) -> String {
    let response = request
        .post("/api/cases")
        .json(&json!({ "title": "Housing benefit appeal" }))
        .await;
    assert_eq!(response.status_code(), 200, "case create should succeed");
    response.json::<Value>()["pid"]
        .as_str()
        .expect("pid in create response")
        .to_string()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// The full round-trip: POST a subject_of link (linked event), GET it,
// DELETE it (unlinked event); assert both events for the case pid.
async fn subject_of_link_create_list_delete_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = create_case(&request).await;
        let person = "person:0c4f1e2a-0000-4000-8000-000000000000";

        // Create the outbound subject_of edge.
        let created = request
            .post(&format!("/api/cases/{pid}/links"))
            .json(&json!({ "kind": "subject_of", "to_ref": person }))
            .await;
        assert_eq!(created.status_code(), 200, "link create should succeed");
        let edge: Value = created.json();
        assert_eq!(edge["kind"], "subject_of");
        assert_eq!(edge["to_ref"], person);
        assert_eq!(edge["from_ref"], format!("case:{pid}"));
        assert_eq!(edge["provenance"], "operator");
        let edge_id = edge["id"].as_str().expect("edge id").to_string();

        // List it back.
        let listed = request.get(&format!("/api/cases/{pid}/links")).await;
        assert_eq!(listed.status_code(), 200);
        let rows: Vec<Value> = listed.json();
        assert_eq!(rows.len(), 1, "one active edge");
        assert_eq!(rows[0]["id"], edge_id);

        // Withdraw it.
        let deleted = request
            .delete(&format!("/api/cases/{pid}/links/{edge_id}"))
            .await;
        assert_eq!(deleted.status_code(), 200, "link delete should succeed");

        // ...and it is gone from the active list.
        let after: Vec<Value> = request.get(&format!("/api/cases/{pid}/links")).await.json();
        assert!(after.is_empty(), "no active edges after withdraw");

        // Both a `linked` and an `unlinked` event were emitted for the pid.
        let events: Vec<Value> = request.get("/api/cases/events/recent").await.json();
        let kinds: Vec<&str> = events
            .iter()
            .filter(|e| e["pid"] == pid)
            .filter_map(|e| e["kind"].as_str())
            .collect();
        assert!(
            kinds.contains(&"linked"),
            "a linked event was emitted, saw {kinds:?}"
        );
        assert!(
            kinds.contains(&"unlinked"),
            "an unlinked event was emitted, saw {kinds:?}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// A disallowed edge kind/endpoint (same_identity from a case) is a 422.
async fn invalid_edge_kind_is_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = create_case(&request).await;
        let response = request
            .post(&format!("/api/cases/{pid}/links"))
            .json(&json!({
                "kind": "same_identity",
                "to_ref": "person:0c4f1e2a-0000-4000-8000-000000000000"
            }))
            .await;
        assert_eq!(
            response.status_code(),
            422,
            "case cannot originate same_identity"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Re-asserting the same edge is idempotent: the same edge id comes back
// and the active list still holds exactly one row.
async fn upsert_is_idempotent() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = create_case(&request).await;
        let person = "person:0c4f1e2a-0000-4000-8000-000000000000";
        let body = json!({ "kind": "subject_of", "to_ref": person });

        let first: Value = request
            .post(&format!("/api/cases/{pid}/links"))
            .json(&body)
            .await
            .json();
        let second: Value = request
            .post(&format!("/api/cases/{pid}/links"))
            .json(&body)
            .await
            .json();
        assert_eq!(first["id"], second["id"], "stable edge id on re-assert");

        let rows: Vec<Value> = request.get(&format!("/api/cases/{pid}/links")).await.json();
        assert_eq!(rows.len(), 1, "re-assert does not duplicate");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// GET /api/cases/links returns every active edge in the canonical §4.2
// shape (edge_id / edge_kind / from_ref=case:<pid>), for reconciliation.
async fn bulk_links_returns_the_canonical_edge_shape() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = create_case(&request).await;
        let to_ref = "person:0c4f1e2a-0000-4000-8000-000000000009";
        let created = request
            .post(&format!("/api/cases/{pid}/links"))
            .json(&json!({ "kind": "subject_of", "to_ref": to_ref }))
            .await;
        assert_eq!(created.status_code(), 200, "link create should succeed");

        let body: Value = request.get("/api/cases/links").await.json();
        let edges = body["edges"].as_array().expect("edges array");
        assert_eq!(edges.len(), 1, "one active edge across all cases");
        let e = &edges[0];
        // Canonical §4.2 field names — deserializable as the aggregator's
        // LinkedEvent (edge_id/edge_kind, not the LinkView id/kind).
        assert!(e["edge_id"].is_string());
        assert_eq!(e["edge_kind"], "subject_of");
        assert_eq!(e["from_ref"], format!("case:{pid}"));
        assert_eq!(e["to_ref"], to_ref);
        assert_eq!(e["provenance"], "operator");
    })
    .await;
}
