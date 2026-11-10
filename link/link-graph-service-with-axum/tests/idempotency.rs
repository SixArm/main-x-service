//! DB-gated proof that [`apply_event_idempotent`] is safe under
//! **at-least-once** bus delivery (event-bus.md §6; spec §10.3, BUS-2):
//! a redelivered envelope (same `event_id`) does not re-apply, and an
//! envelope with no `event_id` (dedup is impossible) is applied every
//! time, exactly as [`apply_event`] alone would.
//!
//! `#[ignore]`d so the default `cargo test` stays green on a
//! database-less machine:
//!
//! ```sh
//! DATABASE_URL=postgres://loco:loco@localhost:5432/link_graph_service_test \
//!   cargo test --test idempotency -- --ignored
//! ```

use axa_test::prelude::*;
use link_graph_service::app::App;
use link_graph_service::events::{Envelope, apply_event_idempotent};
use link_graph_service::models::{edges, entity_presence, processed_events};
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

fn u(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn linked_env(event_id: Uuid, edge_id: Uuid, from: &str, to: &str, seq: i64) -> Envelope {
    let from_entity = from.split(':').next().unwrap();
    serde_json::from_value(json!({
        "entity": from_entity,
        "pid": from.split(':').nth(1).unwrap(),
        "kind": "linked",
        "event_id": event_id.to_string(),
        "seq": seq,
        "occurred_at": "2026-08-03T10:00:00Z",
        "data": {
            "edge_id": edge_id.to_string(),
            "from_ref": from,
            "to_ref": to,
            "edge_kind": "works_at",
            "provenance": "operator"
        }
    }))
    .unwrap()
}

fn presence_env_no_event_id(entity: &str, pid: Uuid, kind: &str, seq: i64) -> Envelope {
    serde_json::from_value(json!({
        "entity": entity,
        "pid": pid.to_string(),
        "kind": kind,
        "seq": seq,
        "occurred_at": "2026-08-03T10:00:00Z"
    }))
    .unwrap()
}

/// A redelivered `linked` event (same `event_id`) is applied once: the
/// second call is a no-op, not a duplicate edge and not an error.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test idempotency -- --ignored`"]
async fn redelivered_linked_event_does_not_duplicate() {
    request::<App, _, _>(|_request, ctx| async move {
        let event_id = u(500);
        let edge_id = u(501);
        let from = format!("person:{}", u(1));
        let to = format!("organization:{}", u(2));

        apply_event_idempotent(&ctx.db, linked_env(event_id, edge_id, &from, &to, 1))
            .await
            .expect("first apply");
        apply_event_idempotent(&ctx.db, linked_env(event_id, edge_id, &from, &to, 1))
            .await
            .expect("redelivered apply must not error");

        let count = edges::Model::all_edge_ids(&ctx.db)
            .await
            .expect("load edges")
            .len();
        assert_eq!(
            count, 1,
            "the redelivered event must not duplicate the edge"
        );

        assert!(
            processed_events::Model::is_processed(&ctx.db, event_id)
                .await
                .expect("query processed_events"),
            "the event_id must be recorded as processed"
        );
    })
    .await;
}

/// Two *different* event_ids for the same edge_id both apply (they are
/// not redeliveries of each other) — the dedup key is `event_id`, not
/// `edge_id`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test idempotency -- --ignored`"]
async fn distinct_event_ids_are_not_deduplicated_against_each_other() {
    request::<App, _, _>(|_request, ctx| async move {
        let edge_id = u(601);
        let from = format!("person:{}", u(11));
        let to = format!("organization:{}", u(12));

        apply_event_idempotent(&ctx.db, linked_env(u(602), edge_id, &from, &to, 1))
            .await
            .expect("first event_id applies");
        apply_event_idempotent(&ctx.db, linked_env(u(603), edge_id, &from, &to, 2))
            .await
            .expect("second, distinct event_id also applies");

        assert!(
            processed_events::Model::is_processed(&ctx.db, u(602))
                .await
                .unwrap()
        );
        assert!(
            processed_events::Model::is_processed(&ctx.db, u(603))
                .await
                .unwrap()
        );
    })
    .await;
}

/// An envelope with no `event_id` cannot be deduped and is applied every
/// time — the upsert underneath (`entity_presence`) makes a repeat apply
/// harmless rather than corrupting.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test idempotency -- --ignored`"]
async fn envelope_with_no_event_id_applies_every_time() {
    request::<App, _, _>(|_request, ctx| async move {
        let pid = u(700);
        apply_event_idempotent(
            &ctx.db,
            presence_env_no_event_id("person", pid, "created", 1),
        )
        .await
        .expect("first apply");
        apply_event_idempotent(
            &ctx.db,
            presence_env_no_event_id("person", pid, "created", 2),
        )
        .await
        .expect("second apply, no event_id to dedupe on");

        let ref_str = format!("person:{pid}");
        let row = entity_presence::Model::find(&ctx.db, &ref_str)
            .await
            .expect("query entity_presence")
            .expect("row exists");
        assert!(row.alive);
        // The upsert advanced to the later seq — not corrupted by the
        // repeat apply.
        assert_eq!(row.last_seq, 2);
    })
    .await;
}

/// `db truncate` clears the dedupe table with the read-model (LG-T1).
/// Left behind, a truncated database would skip a replayed event whose
/// effect it no longer holds, so a replay from offset 0 could never
/// rebuild the graph.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test idempotency -- --ignored`"]
async fn truncate_clears_processed_events_so_a_replay_reapplies() {
    request::<App, _, _>(|_request, ctx| async move {
        let event_id = u(801);
        let edge_id = u(802);
        let from = format!("person:{}", u(803));
        let to = format!("organization:{}", u(804));
        apply_event_idempotent(&ctx.db, linked_env(event_id, edge_id, &from, &to, 1))
            .await
            .expect("first apply");
        assert!(
            processed_events::Model::is_processed(&ctx.db, event_id)
                .await
                .expect("query processed_events")
        );

        App::truncate(&ctx).await.expect("truncate");
        assert!(
            !processed_events::Model::is_processed(&ctx.db, event_id)
                .await
                .expect("query processed_events"),
            "truncate must clear processed_events"
        );

        // The same event, replayed, rebuilds the edge.
        apply_event_idempotent(&ctx.db, linked_env(event_id, edge_id, &from, &to, 1))
            .await
            .expect("replayed apply");
        assert!(
            edges::Model::find(&ctx.db, edge_id)
                .await
                .expect("query edges")
                .is_some(),
            "a replay after truncate must re-apply"
        );
    })
    .await;
}
