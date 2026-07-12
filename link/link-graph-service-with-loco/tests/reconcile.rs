//! DB-gated test for reconciliation (spec T-20 / design §8): a **mock**
//! authoritative source drives the diff + repair against the DB — an
//! authoritative edge missing from the read-model is added, and a
//! read-model edge absent from the authoritative set is removed. The pure
//! diff is additionally unit-tested in `src/reconcile.rs`.
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test reconcile -- --ignored`.

use entity_ref::{EdgeKind, EntityRef, EntityType};
use link_graph_service::app::App;
use link_graph_service::events::{Envelope, LinkedEvent, apply_event};
use link_graph_service::reconcile::{self, AuthoritativeSource};
use loco_rs::prelude::ModelResult;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

fn u(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

/// A mock authoritative source for `entity` returning a fixed edge set.
struct MockSource(EntityType, Vec<LinkedEvent>);

#[async_trait::async_trait]
impl AuthoritativeSource for MockSource {
    fn entity(&self) -> EntityType {
        self.0
    }
    async fn fetch_all(&self) -> ModelResult<Vec<LinkedEvent>> {
        Ok(self.1.clone())
    }
}

/// An authoritative `subject_of` edge (case → person).
fn auth_edge(edge_id: u128, case: u128, person: u128) -> LinkedEvent {
    LinkedEvent {
        edge_id: u(edge_id),
        from_ref: EntityRef::new(EntityType::Case, u(case)),
        to_ref: EntityRef::new(EntityType::Person, u(person)),
        edge_kind: EdgeKind::SubjectOf,
        role: None,
        confidence: None,
        provenance: "operator".into(),
        valid_from: None,
        valid_to: None,
    }
}

fn linked_env(edge_id: Uuid, from: &str, to: &str) -> Envelope {
    linked_env_kind(edge_id, from, to, "subject_of")
}

fn same_identity_env(edge_id: Uuid, from: &str, to: &str) -> Envelope {
    linked_env_kind(edge_id, from, to, "same_identity")
}

fn linked_env_kind(edge_id: Uuid, from: &str, to: &str, edge_kind: &str) -> Envelope {
    serde_json::from_value(json!({
        "entity": from.split(':').next().unwrap(),
        "pid": from.split(':').nth(1).unwrap(),
        "kind": "linked",
        "seq": 1,
        "occurred_at": "2026-07-10T10:00:00Z",
        "data": {
            "edge_id": edge_id.to_string(), "from_ref": from, "to_ref": to,
            "edge_kind": edge_kind, "provenance": "operator"
        }
    }))
    .unwrap()
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test reconcile -- --ignored`"]
async fn reconcile_adds_missing_and_removes_extra() {
    request::<App, _, _>(|request, ctx| async move {
        // Read-model starts with edge A (case:1 → person:2).
        apply_event(
            &ctx.db,
            linked_env(
                u(10),
                &format!("case:{}", u(1)),
                &format!("person:{}", u(2)),
            ),
        )
        .await
        .unwrap();

        // Authoritative source has edge B (case:3 → person:4) but NOT A —
        // so A is extra (remove) and B is missing (add): divergence 2.
        let source = MockSource(EntityType::Case, vec![auth_edge(20, 3, 4)]);
        let divergence = reconcile::reconcile(&ctx.db, &source).await.unwrap();
        assert_eq!(divergence, 2, "one missing + one extra");

        // The read-model now matches the authoritative set: only edge B.
        let body: Value = request.get("/api/edges").await.json();
        let edges = body["data"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1, "repaired to the authoritative set");
        assert_eq!(edges[0]["edge_id"], u(20).to_string());
        assert_eq!(edges[0]["to_ref"], format!("person:{}", u(4)));

        // A second pass finds nothing to reconcile.
        let again = reconcile::reconcile(&ctx.db, &source).await.unwrap();
        assert_eq!(again, 0, "converged — no divergence on re-run");
    })
    .await;
}

/// SEC-B1: a per-entity reconcile pass must only touch its own entity's
/// edges. The `case` source diffs against the case-scoped read-model, so a
/// `person` `same_identity` edge is invisible to it and survives — before
/// the fix, reconcile diffed against the *global* set and the case pass
/// deleted every person edge (the graph never converged).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test reconcile -- --ignored`"]
async fn reconcile_is_scoped_to_the_source_entity() {
    request::<App, _, _>(|request, ctx| async move {
        // Read-model holds a case edge AND a person (same_identity) edge.
        apply_event(
            &ctx.db,
            linked_env(
                u(10),
                &format!("case:{}", u(1)),
                &format!("person:{}", u(2)),
            ),
        )
        .await
        .unwrap();
        apply_event(
            &ctx.db,
            same_identity_env(
                u(30),
                &format!("person:{}", u(5)),
                &format!("worker:{}", u(6)),
            ),
        )
        .await
        .unwrap();

        // A CASE source that already matches the case edge: zero divergence,
        // and — crucially — it must NOT delete the person edge.
        let source = MockSource(EntityType::Case, vec![auth_edge(10, 1, 2)]);
        let divergence = reconcile::reconcile(&ctx.db, &source).await.unwrap();
        assert_eq!(divergence, 0, "case edge matches; person edge out of scope");

        // Both edges are still present.
        let body: Value = request.get("/api/edges").await.json();
        let edges = body["data"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 2, "the person edge survived the case pass");
        let ids: Vec<&str> = edges
            .iter()
            .map(|e| e["edge_id"].as_str().unwrap())
            .collect();
        assert!(ids.contains(&u(10).to_string().as_str()));
        assert!(
            ids.contains(&u(30).to_string().as_str()),
            "person same_identity edge must not be reconciled away by the case source"
        );
    })
    .await;
}
