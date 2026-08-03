//! Round-trip the real bus consumer against a **live** Fluvio broker
//! (BUS-2; spec §13 T-6, `agents/share/event-bus.md` §10 "Fluvio-gated"
//! tests).
//!
//! The whole file is gated on the `fluvio` cargo feature (`#![cfg(...)]`)
//! so a default build — and the family CI, which does not pass
//! `--features fluvio` to this crate today — compiles a file with zero
//! tests rather than skipping a feature-locked one. On top of that, the
//! single test is `#[ignore]`d: it needs both Postgres and a live
//! Fluvio broker, neither of which any automated run in this repo stands
//! up today. Run it against `compose.fluvio.yaml`:
//!
//! ```sh
//! podman compose -f compose.fluvio.yaml up -d
//! LINK_GRAPH_FLUVIO_ENDPOINT=127.0.0.1:9103 \
//!   cargo test --features fluvio --test fluvio_consumer -- --ignored
//! podman compose -f compose.fluvio.yaml down -v
//! ```
//!
//! This test exists because everything else about the consumer is
//! verified only by the compiler (`cargo build --features fluvio`
//! confirms the real `fluvio` 0.50 API is used correctly, but not that
//! a produced record actually reaches [`consumer::spawn`]'s stream and
//! gets applied). Saying "the bus consumer is tested" on the strength of
//! the compile check alone would be false — same posture as person's
//! `s3_round_trip_against_a_live_endpoint` (BLK-4) and case's
//! `fluvio_relay_publishes_an_outbox_row_to_a_real_topic` (BUS-1).
//!
//! Drives the already-**public** [`consumer::spawn`] rather than a
//! test-only internal entry point, so the test exercises exactly what
//! `App::after_routes` wires in production — spawning one task per
//! entity topic and observing the effect on the `person` topic.
#![cfg(feature = "fluvio")]

use std::time::Duration;

use fluvio::{Fluvio, FluvioConfig};
use link_graph_service::app::App;
use link_graph_service::consumer;
use link_graph_service::models::edges;
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL AND a live Fluvio broker; see this file's module docs"]
async fn bus_consumer_applies_a_produced_linked_event() {
    let endpoint = std::env::var("LINK_GRAPH_FLUVIO_ENDPOINT")
        .expect("set LINK_GRAPH_FLUVIO_ENDPOINT to the broker's SC address (e.g. 127.0.0.1:9103)");

    request::<App, _, _>(|_request, ctx| async move {
        let edge_id = Uuid::new_v4();
        let event_id = Uuid::new_v4();
        let from_pid = Uuid::new_v4();
        let to_pid = Uuid::new_v4();

        let envelope = json!({
            "entity": "person",
            "pid": from_pid.to_string(),
            "kind": "linked",
            "event_id": event_id.to_string(),
            "seq": 1,
            "occurred_at": "2026-08-03T10:00:00Z",
            "data": {
                "edge_id": edge_id.to_string(),
                "from_ref": format!("person:{from_pid}"),
                "to_ref": format!("organization:{to_pid}"),
                "edge_kind": "works_at",
                "provenance": "operator"
            }
        });

        // Produce directly onto the topic `consumer::spawn` will consume
        // from — this test is the producer side that BUS-1's FluvioSink
        // plays for a real deployment.
        let config = FluvioConfig::new(&endpoint);
        let client = Fluvio::connect_with_config(&config)
            .await
            .expect("connect to the live broker");
        let producer = client
            .topic_producer("mxi.person.events")
            .await
            .expect("open producer");
        producer
            .send(
                from_pid.to_string(),
                serde_json::to_vec(&envelope).expect("encode envelope"),
            )
            .await
            .expect("send");

        // `consumer::spawn` is exactly what `App::after_routes` calls in
        // production — same entry point, same behaviour.
        consumer::spawn(ctx.db.clone());

        // No commit-notification to wait on; a bounded poll is the honest
        // way to observe an async background consumer's effect.
        let mut applied = false;
        for _ in 0..20 {
            tokio::time::sleep(Duration::from_millis(500)).await;
            if edges::Entity::find_by_id(edge_id)
                .one(&ctx.db)
                .await
                .expect("query edges")
                .is_some()
            {
                applied = true;
                break;
            }
        }
        assert!(
            applied,
            "the produced linked event should have been consumed and applied within 10s"
        );
    })
    .await;
}
