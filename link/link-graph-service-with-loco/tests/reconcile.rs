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
use loco_rs::prelude::{ModelError, ModelResult};
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

/// A mock authoritative source for `entity` that always fails to fetch —
/// simulates a timeout / non-2xx / malformed-JSON pass (T-35).
struct FailingSource(EntityType);

#[async_trait::async_trait]
impl AuthoritativeSource for FailingSource {
    fn entity(&self) -> EntityType {
        self.0
    }
    async fn fetch_all(&self) -> ModelResult<Vec<LinkedEvent>> {
        Err(ModelError::Any(Box::new(std::io::Error::other(
            "mock fetch failure",
        ))))
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

/// T-34: a `case` pass's divergence and a `person` pass's divergence are
/// independently readable series — a converged `case` pass (`0`) must not
/// zero out a diverging `person` pass's stale-but-real count, which a
/// single unlabeled gauge could not distinguish.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test reconcile -- --ignored`"]
async fn reconciliation_divergence_gauge_is_independent_per_entity() {
    use link_graph_service::metrics::Metrics;

    request::<App, _, _>(|_request, ctx| async move {
        // A person source diverges by one (nothing in the read-model yet).
        let person_source = MockSource(EntityType::Person, vec![]);
        // Read-model has no person edges, authoritative source has none
        // either in this pass — instead assert the *case* pass below
        // first produces a real divergence, then a converged person pass
        // must not clobber it.
        let case_source = MockSource(EntityType::Case, vec![auth_edge(40, 7, 8)]);
        let case_divergence = reconcile::reconcile(&ctx.db, &case_source).await.unwrap();
        assert_eq!(case_divergence, 1, "case source has one missing edge");

        // A converged person pass (matches nothing, diverges by zero).
        let person_divergence = reconcile::reconcile(&ctx.db, &person_source).await.unwrap();
        assert_eq!(person_divergence, 0, "person source is empty and converged");

        // The two series must be independently readable: case's `1` must
        // survive the person pass's `0`.
        let m = Metrics::global();
        assert_eq!(
            m.reconciliation_divergence
                .with_label_values(&["case"])
                .get(),
            1,
            "case's divergence must not be zeroed by the person pass"
        );
        assert_eq!(
            m.reconciliation_divergence
                .with_label_values(&["person"])
                .get(),
            0
        );
    })
    .await;
}

/// T-35: a failed reconciliation pass leaves the last-success gauge
/// exactly where it was — a caller can distinguish "converged N seconds
/// ago" from "just failed" instead of the gauge alone looking identical
/// to "never run".
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test reconcile -- --ignored`"]
async fn reconciliation_last_success_gauge_is_unchanged_by_a_failed_pass() {
    use link_graph_service::metrics::Metrics;

    request::<App, _, _>(|_request, ctx| async move {
        // `auth_edge` always builds a case→person edge, so the source
        // entity here is `Case` (matching `edge_valid_for_source`'s
        // from_ref check) — the label under test, not the specific
        // entity, is what T-35 cares about.
        let entity = EntityType::Case;
        let m = Metrics::global();

        // A successful pass with a real (non-zero) divergence advances the
        // last-success gauge and records that divergence.
        let ok_source = MockSource(entity, vec![auth_edge(50, 9, 10)]);
        let first = reconcile::reconcile(&ctx.db, &ok_source).await.unwrap();
        assert_eq!(first, 1, "one missing edge");
        let after_success = m
            .reconciliation_last_success_unixtime
            .with_label_values(&["case"])
            .get();
        assert!(after_success > 0, "a successful pass must set the gauge");
        assert_eq!(
            m.reconciliation_divergence
                .with_label_values(&["case"])
                .get(),
            1
        );

        // A failing pass must not advance the last-success gauge, and must
        // not touch the divergence gauge either — reconcile returns before
        // setting either on a fetch error, so the prior pass's real `1`
        // survives rather than being reset to `0`.
        let failing_source = FailingSource(entity);
        let err = reconcile::reconcile(&ctx.db, &failing_source).await;
        assert!(err.is_err(), "the mock fetch is designed to fail");
        assert_eq!(
            m.reconciliation_last_success_unixtime
                .with_label_values(&["case"])
                .get(),
            after_success,
            "a failed pass must not advance the last-success gauge"
        );
        assert_eq!(
            m.reconciliation_divergence
                .with_label_values(&["case"])
                .get(),
            1,
            "a failed pass must not reset the prior pass's real divergence count"
        );
    })
    .await;
}
