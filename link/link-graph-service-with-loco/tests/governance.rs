//! DB-gated proof that with governance active (`LINK_GRAPH_REQUIRE_AUTH`),
//! a caller without case-read authorisation cannot see — or learn of —
//! `subject_of` (case↔person) edges (spec T-16 / design §10), while
//! ordinary affiliation edges stay visible.
//!
//! Runs in its **own test binary** so the process-wide `require_auth`
//! `OnceLock` — pinned on here — does not leak into the enforcement-off
//! read suite in `tests/graph_endpoints.rs`. The `may_see_governed` /
//! `conceal_governed` decision logic is additionally unit-pinned DB-free
//! in `src/auth.rs`.
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test governance -- --ignored`.

use link_graph_service::app::App;
use link_graph_service::events::{Envelope, apply_event};
use link_graph_service::models::_entities::audit_log;
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

fn u(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

/// Build a `linked` envelope for `from --edge_kind--> to`.
fn linked_env(edge_id: Uuid, from: &str, to: &str, edge_kind: &str, seq: i64) -> Envelope {
    let from_entity = from.split(':').next().unwrap();
    serde_json::from_value(json!({
        "entity": from_entity,
        "pid": from.split(':').nth(1).unwrap(),
        "kind": "linked",
        "seq": seq,
        "occurred_at": "2026-07-09T10:00:00Z",
        "data": {
            "edge_id": edge_id.to_string(),
            "from_ref": from,
            "to_ref": to,
            "edge_kind": edge_kind,
            "provenance": "operator"
        }
    }))
    .unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test governance -- --ignored`"]
async fn governed_edges_are_concealed_from_an_unauthorised_caller() {
    // Activate governance BEFORE the app boots (read once into a OnceLock).
    // `set_var` is `unsafe` in edition 2024; single-threaded test setup.
    unsafe {
        std::env::set_var("LINK_GRAPH_REQUIRE_AUTH", "1");
    }

    request::<App, _, _>(|request, ctx| async move {
        let case = format!("case:{}", u(1));
        let person = format!("person:{}", u(2));
        let worker = format!("worker:{}", u(3));
        let org = format!("organization:{}", u(4));

        // A governed case↔person edge and an ordinary affiliation.
        apply_event(&ctx.db, linked_env(u(10), &case, &person, "subject_of", 1))
            .await
            .unwrap();
        apply_event(&ctx.db, linked_env(u(11), &worker, &org, "employed_by", 2))
            .await
            .unwrap();

        // No bearer token ⇒ unauthorised. `/edges` hides the subject_of
        // edge but still shows the affiliation.
        let body: Value = request.get("/api/edges").await.json();
        let edges = body["data"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1, "only the non-governed edge is visible");
        assert_eq!(edges[0]["kind"], "employed_by");
        assert!(
            !edges.iter().any(|e| e["kind"] == "subject_of"),
            "the case↔person edge must not appear"
        );

        // Even a direct kind filter conceals existence (empty, not 403).
        let direct: Value = request.get("/api/edges?kind=subject_of").await.json();
        assert_eq!(direct["success"], true);
        assert_eq!(
            direct["data"]["edges"].as_array().unwrap().len(),
            0,
            "a direct subject_of query reveals nothing"
        );

        // Neighbors of the case conceals the edge too.
        let nbrs: Value = request.get(&format!("/api/neighbors/{case}")).await.json();
        assert_eq!(
            nbrs["data"]["edges"].as_array().unwrap().len(),
            0,
            "the case's own subject_of edge is concealed"
        );

        // The governed WRITE was audited (apply_linked, no actor); the
        // concealed reads surfaced nothing, so no read_edge row exists.
        let audits = audit_log::Entity::find().all(&ctx.db).await.unwrap();
        assert_eq!(audits.len(), 1, "only the subject_of apply_linked audited");
        assert_eq!(audits[0].action, "apply_linked");
        assert_eq!(audits[0].edge_kind.as_deref(), Some("subject_of"));
        assert!(audits[0].actor.is_none(), "a bus-driven write has no actor");
    })
    .await;
}
