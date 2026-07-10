//! DB-gated test for lazy verify-on-read (spec T-10 / design §5.1): a
//! **mock** probe resolves unknown endpoint presence, which caches it in
//! `entity_presence` and recomputes the incident edge status
//! (`unverified` → `verified` / `dangling`). This exercises the DB path
//! without any HTTP; the real `HttpPresenceProbe` is compile-checked and
//! the URL resolution is unit-tested in `src/probe.rs`.
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test lazy_verify -- --ignored`.

use entity_ref::{EntityRef, EntityType};
use link_graph_service::app::App;
use link_graph_service::events::{Envelope, apply_event};
use link_graph_service::probe::{self, PresenceProbe, ProbeOutcome};
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

/// A probe that returns the same fixed outcome for every ref.
struct MockProbe(ProbeOutcome);

#[async_trait::async_trait]
impl PresenceProbe for MockProbe {
    async fn probe(&self, _r: &EntityRef) -> ProbeOutcome {
        self.0
    }
}

fn u(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn works_at(edge_id: Uuid, person: &str, org: &str) -> Envelope {
    serde_json::from_value(json!({
        "entity": "person",
        "pid": person.split(':').nth(1).unwrap(),
        "kind": "linked",
        "seq": 1,
        "occurred_at": "2026-07-10T10:00:00Z",
        "data": {
            "edge_id": edge_id.to_string(), "from_ref": person, "to_ref": org,
            "edge_kind": "works_at", "provenance": "operator"
        }
    }))
    .unwrap()
}

fn endpoints() -> Vec<EntityRef> {
    vec![
        EntityRef::new(EntityType::Person, u(1)),
        EntityRef::new(EntityType::Organization, u(2)),
    ]
}

fn status_count(body: &Value) -> usize {
    body["data"]["edges"].as_array().unwrap().len()
}

/// Both endpoints resolve alive ⇒ the unverified edge settles to verified.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test lazy_verify -- --ignored`"]
async fn alive_endpoints_settle_the_edge_to_verified() {
    request::<App, _, _>(|request, ctx| async move {
        let (person, org) = (format!("person:{}", u(1)), format!("organization:{}", u(2)));
        apply_event(&ctx.db, works_at(u(10), &person, &org))
            .await
            .unwrap();

        // No presence observed yet ⇒ unverified.
        let body: Value = request.get("/api/edges?status=unverified").await.json();
        assert_eq!(status_count(&body), 1);

        // A probe that says both endpoints are alive resolves both.
        let resolved =
            probe::verify_unknown(&ctx.db, &MockProbe(ProbeOutcome::Alive), &endpoints())
                .await
                .unwrap();
        assert_eq!(resolved, 2, "both endpoints newly resolved");

        // The edge is now verified.
        let body: Value = request.get("/api/edges?status=verified").await.json();
        assert_eq!(status_count(&body), 1, "both alive => verified");

        // A second call is a no-op — presence is already known.
        let again = probe::verify_unknown(&ctx.db, &MockProbe(ProbeOutcome::Absent), &endpoints())
            .await
            .unwrap();
        assert_eq!(again, 0, "already-known endpoints are not re-probed");
    })
    .await;
}

/// An endpoint resolved absent (404 at the source) dangles the edge.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test lazy_verify -- --ignored`"]
async fn an_absent_endpoint_dangles_the_edge() {
    request::<App, _, _>(|request, ctx| async move {
        let (person, org) = (format!("person:{}", u(1)), format!("organization:{}", u(2)));
        apply_event(&ctx.db, works_at(u(20), &person, &org))
            .await
            .unwrap();

        probe::verify_unknown(&ctx.db, &MockProbe(ProbeOutcome::Absent), &endpoints())
            .await
            .unwrap();

        let body: Value = request.get("/api/edges?status=dangling").await.json();
        assert_eq!(status_count(&body), 1, "an absent endpoint => dangling");
    })
    .await;
}
