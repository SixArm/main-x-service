//! DB-gated request-level integration tests (spec §11.2). They boot the
//! app against the `test` environment (needs a reachable PostgreSQL,
//! `config/test.yaml`; override with `DATABASE_URL`), drive the internal
//! [`apply_event`] seam to populate the read-model, and assert on the
//! read endpoints. `#[ignore]`d so the default `cargo test` stays green
//! on a database-less machine:
//!
//! ```sh
//! DATABASE_URL=postgres://loco:loco@localhost:5432/link_graph_service_test \
//!   cargo test -- --ignored
//! ```

use link_graph_service::app::App;
use link_graph_service::events::{Envelope, apply_event};
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

/// A stable UUID for a given small integer, for readable fixtures.
fn u(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

/// Build a `created` / `deleted` presence envelope.
fn presence_env(entity: &str, pid: Uuid, kind: &str, seq: i64) -> Envelope {
    serde_json::from_value(json!({
        "entity": entity,
        "pid": pid.to_string(),
        "kind": kind,
        "seq": seq,
        "occurred_at": "2026-07-09T10:00:00Z"
    }))
    .unwrap()
}

/// Build a `linked` envelope for `from --edge_kind--> to`.
fn linked_env(
    edge_id: Uuid,
    from: &str,
    to: &str,
    edge_kind: &str,
    provenance: &str,
    seq: i64,
) -> Envelope {
    let from_entity = from.split(':').next().unwrap();
    serde_json::from_value(json!({
        "entity": from_entity,
        "pid": from.split(':').nth(1).unwrap(),
        "kind": "linked",
        "event_id": Uuid::from_u128(9000 + seq as u128).to_string(),
        "seq": seq,
        "occurred_at": "2026-07-09T10:05:00Z",
        "data": {
            "edge_id": edge_id.to_string(),
            "from_ref": from,
            "to_ref": to,
            "edge_kind": edge_kind,
            "provenance": provenance
        }
    }))
    .unwrap()
}

/// A `linked` edge is projected, then reachable via `neighbors` and
/// `edges`; every graph response carries an `as_of` watermark.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn linked_edge_is_projected_and_readable() {
    request::<App, _, _>(|request, ctx| async move {
        let person = format!("person:{}", u(1));
        let org = format!("organization:{}", u(2));
        let edge_id = u(100);
        apply_event(
            &ctx.db,
            linked_env(edge_id, &person, &org, "works_at", "operator", 1),
        )
        .await
        .expect("apply linked");

        // neighbors of the person surfaces the edge, with as_of present.
        let resp = request.get(&format!("/api/neighbors/{person}")).await;
        assert_eq!(resp.status_code(), 200);
        let body: Value = resp.json();
        assert_eq!(body["success"], true);
        assert_eq!(body["data"]["edges"].as_array().unwrap().len(), 1);
        assert_eq!(body["data"]["edges"][0]["kind"], "works_at");
        assert!(
            body["data"].get("as_of").is_some(),
            "graph response carries as_of"
        );

        // edges filter by kind also finds it.
        let resp = request.get("/api/edges?kind=works_at").await;
        let body: Value = resp.json();
        assert_eq!(body["data"]["edges"].as_array().unwrap().len(), 1);
    })
    .await;
}

/// A symmetric `same_identity` edge is stored once, canonicalised with
/// the smaller URN as `from_ref` regardless of assertion order.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn same_identity_is_canonicalised_once() {
    request::<App, _, _>(|request, ctx| async move {
        let person = format!("person:{}", u(1));
        let worker = format!("worker:{}", u(1));
        // Assert worker -> person; canonical stores person as `from`.
        apply_event(
            &ctx.db,
            linked_env(u(200), &worker, &person, "same_identity", "operator", 1),
        )
        .await
        .expect("apply linked");

        let resp = request.get("/api/edges?kind=same_identity").await;
        let body: Value = resp.json();
        let edges = body["data"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(
            edges[0]["from_ref"], person,
            "smaller URN is canonical from"
        );
        assert_eq!(edges[0]["directed"], false);
    })
    .await;
}

/// Presence drives the integrity lifecycle: both endpoints alive ⇒
/// verified; deleting an endpoint flips the edge to dangling.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn presence_drives_status_lifecycle() {
    request::<App, _, _>(|request, ctx| async move {
        let person = format!("person:{}", u(1));
        let org = format!("organization:{}", u(2));
        apply_event(&ctx.db, presence_env("person", u(1), "created", 1))
            .await
            .unwrap();
        apply_event(&ctx.db, presence_env("organization", u(2), "created", 1))
            .await
            .unwrap();
        apply_event(
            &ctx.db,
            linked_env(u(300), &person, &org, "works_at", "operator", 2),
        )
        .await
        .unwrap();

        let body: Value = request.get("/api/edges?status=verified").await.json();
        assert_eq!(
            body["data"]["edges"].as_array().unwrap().len(),
            1,
            "both alive => verified"
        );

        // Delete the org: the incident edge flips to dangling.
        apply_event(&ctx.db, presence_env("organization", u(2), "deleted", 3))
            .await
            .unwrap();
        let body: Value = request.get("/api/edges?status=dangling").await.json();
        assert_eq!(
            body["data"]["edges"].as_array().unwrap().len(),
            1,
            "deleted endpoint => dangling"
        );
    })
    .await;
}

/// `single-view` unifies person ↔ worker via `same_identity` and derives
/// the worker's employer for the unified identity.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn single_view_unifies_and_derives_employer() {
    request::<App, _, _>(|request, ctx| async move {
        let person = format!("person:{}", u(1));
        let worker = format!("worker:{}", u(1));
        let org = format!("organization:{}", u(2));
        apply_event(
            &ctx.db,
            linked_env(u(400), &person, &worker, "same_identity", "operator", 1),
        )
        .await
        .unwrap();
        apply_event(
            &ctx.db,
            linked_env(u(401), &worker, &org, "employed_by", "operator", 2),
        )
        .await
        .unwrap();

        let body: Value = request
            .get(&format!("/api/single-view/{person}"))
            .await
            .json();
        let ids = body["data"]["identity_refs"].as_array().unwrap();
        assert_eq!(ids.len(), 2, "person + worker unified");
        let affs = body["data"]["affiliations"].as_array().unwrap();
        assert_eq!(affs.len(), 1);
        assert_eq!(affs[0]["kind"], "employed_by");
        assert_eq!(affs[0]["to"], org);
    })
    .await;
}

/// `unlinked` removes the edge (idempotently).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn unlinked_removes_the_edge() {
    request::<App, _, _>(|request, ctx| async move {
        let person = format!("person:{}", u(1));
        let org = format!("organization:{}", u(2));
        let edge_id = u(500);
        apply_event(
            &ctx.db,
            linked_env(edge_id, &person, &org, "works_at", "operator", 1),
        )
        .await
        .unwrap();

        let unlinked: Envelope = serde_json::from_value(json!({
            "entity": "person",
            "pid": u(1).to_string(),
            "kind": "unlinked",
            "seq": 2,
            "occurred_at": "2026-07-09T11:00:00Z",
            "data": { "edge_id": edge_id.to_string() }
        }))
        .unwrap();
        apply_event(&ctx.db, unlinked).await.unwrap();

        let body: Value = request.get("/api/edges").await.json();
        assert_eq!(
            body["data"]["edges"].as_array().unwrap().len(),
            0,
            "edge removed"
        );
    })
    .await;
}

/// The freshness endpoint reports a per-topic watermark after events.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn freshness_reports_consumed_topics() {
    request::<App, _, _>(|request, ctx| async move {
        apply_event(&ctx.db, presence_env("person", u(1), "created", 1))
            .await
            .unwrap();
        let body: Value = request.get("/api/health/freshness").await.json();
        let topics = body["data"]["topics"].as_array().unwrap();
        assert!(
            topics.iter().any(|t| t["entity"] == "person"),
            "person topic present"
        );
        assert!(body["data"].get("as_of").is_some());
    })
    .await;
}

/// Malformed refs, unknown kinds, and over-cap depth are `400`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn bad_requests_are_rejected() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(
            request.get("/api/neighbors/not-a-ref").await.status_code(),
            400
        );
        assert_eq!(
            request
                .get(&format!("/api/neighbors/person:{}?kind=nope", u(1)))
                .await
                .status_code(),
            400
        );
        assert_eq!(
            request
                .get(&format!("/api/neighbors/person:{}?depth=5", u(1)))
                .await
                .status_code(),
            400
        );
    })
    .await;
}

/// Build a `merged` envelope: `merged_from` (the duplicate) folds into
/// the survivor named by `entity`+`pid`.
fn merged_env(entity: &str, survivor_pid: Uuid, merged_from: Uuid, seq: i64) -> Envelope {
    serde_json::from_value(json!({
        "entity": entity,
        "pid": survivor_pid.to_string(),
        "kind": "merged",
        "seq": seq,
        "occurred_at": "2026-07-09T12:00:00Z",
        "data": { "merged_from": merged_from.to_string() }
    }))
    .unwrap()
}

/// A `merged` event repoints the duplicate's edges onto the survivor
/// (spec §5.3 / T-9): the edge follows the merge, and querying the
/// survivor surfaces it while the merged-away ref has none.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merged_repoints_edges_onto_the_survivor() {
    request::<App, _, _>(|request, ctx| async move {
        let dup = format!("person:{}", u(1)); // the duplicate
        let survivor = format!("person:{}", u(2));
        let org = format!("organization:{}", u(9));
        // The duplicate works_at org.
        apply_event(
            &ctx.db,
            linked_env(u(600), &dup, &org, "works_at", "operator", 1),
        )
        .await
        .unwrap();

        // person:1 is merged into person:2.
        apply_event(&ctx.db, merged_env("person", u(2), u(1), 2))
            .await
            .unwrap();

        // The survivor now carries the edge...
        let body: Value = request
            .get(&format!("/api/neighbors/{survivor}"))
            .await
            .json();
        let edges = body["data"]["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1, "edge repointed to the survivor");
        assert_eq!(edges[0]["from_ref"], survivor);
        assert_eq!(edges[0]["to_ref"], org);

        // ...and the merged-away ref carries none.
        let gone: Value = request.get(&format!("/api/neighbors/{dup}")).await.json();
        assert_eq!(
            gone["data"]["edges"].as_array().unwrap().len(),
            0,
            "the duplicate has no edges after the merge"
        );
    })
    .await;
}

/// Repointing de-duplicates: when the duplicate and the survivor both
/// have the same affiliation, the merge leaves exactly one edge.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merged_dedups_a_colliding_edge() {
    request::<App, _, _>(|request, ctx| async move {
        let dup = format!("person:{}", u(1));
        let survivor = format!("person:{}", u(2));
        let org = format!("organization:{}", u(9));
        // Both the duplicate and the survivor already work_at the org.
        apply_event(
            &ctx.db,
            linked_env(u(700), &dup, &org, "works_at", "operator", 1),
        )
        .await
        .unwrap();
        apply_event(
            &ctx.db,
            linked_env(u(701), &survivor, &org, "works_at", "operator", 2),
        )
        .await
        .unwrap();

        apply_event(&ctx.db, merged_env("person", u(2), u(1), 3))
            .await
            .unwrap();

        let body: Value = request
            .get(&format!("/api/neighbors/{survivor}"))
            .await
            .json();
        assert_eq!(
            body["data"]["edges"].as_array().unwrap().len(),
            1,
            "the colliding edge was de-duplicated, not doubled"
        );
    })
    .await;
}
