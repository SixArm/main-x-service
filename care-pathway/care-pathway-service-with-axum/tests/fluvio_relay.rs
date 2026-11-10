//! Round-trip the durable-bus relay against a **real** Fluvio broker
//! (BUS-3; `agents/share/event-bus.md` §10 "Fluvio-gated" tests).
//!
//! The whole file is gated on the `fluvio` cargo feature (`#![cfg(...)]`)
//! so a default build — and the family CI, which does not pass
//! `--features fluvio` to this crate today — compiles a file with zero
//! tests rather than skipping a feature-locked one. On top of that, the
//! single test is `#[ignore]`d: it needs both Postgres and a live Fluvio
//! broker, neither of which any automated run in this repo stands up
//! today. Run it against `compose.fluvio.yaml`:
//!
//! ```sh
//! podman compose -f compose.fluvio.yaml up -d
//! CARE_PATHWAY_FLUVIO_ENDPOINT=127.0.0.1:9103 \
//!   cargo test --features fluvio --test fluvio_relay -- --ignored
//! podman compose -f compose.fluvio.yaml down -v
//! ```
//!
//! This test exists because everything else about `FluvioSink` is
//! verified only by the compiler (`cargo build --features fluvio`
//! confirms the real `fluvio` 0.50 API is used correctly, but not that a
//! `send` actually reaches a topic). Saying "the Fluvio sink is tested"
//! on the strength of the compile check alone would be false — same
//! posture as case's `fluvio_relay_publishes_an_outbox_row_to_a_real_topic`
//! (`case/case-service-with-axum/tests/fluvio_relay.rs`, BUS-1) and
//! person's `s3_round_trip_against_a_live_endpoint`
//! (`person/person-service-with-axum/src/bulk/store.rs`, BLK-4).
#![cfg(feature = "fluvio")]

use axa_test::prelude::*;
use care_pathway_matcher::CarePathway;
use care_pathway_service::app::App;
use care_pathway_service::models::event_outbox;
use care_pathway_service::relay::{self, FluvioSink};
use care_pathway_service::streaming;
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL AND a live Fluvio broker; see this file's module docs"]
async fn fluvio_relay_publishes_an_outbox_row_to_a_real_topic() {
    let endpoint = std::env::var("CARE_PATHWAY_FLUVIO_ENDPOINT").expect(
        "set CARE_PATHWAY_FLUVIO_ENDPOINT to the broker's SC address (e.g. 127.0.0.1:9103)",
    );

    // Durable transport, so `create_and_emit` writes an outbox row.
    unsafe {
        // Keep this binary's Tantivy index out of the working directory
        // and its boot rebuild out of the way (see tests/requests/mod.rs).
        std::env::set_var(
            "CARE_PATHWAY_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!(
                "care-pathway-fluvio-relay-index-{}",
                std::process::id()
            )),
        );
        std::env::set_var("CARE_PATHWAY_SEARCH_BOOT_REINDEX", "0");
        std::env::set_var("CARE_PATHWAY_EVENT_TRANSPORT", "outbox");
    }

    request::<App, _, _>(|_request, ctx| async move {
        let pathway = CarePathway::new("Fluvio Relay Round Trip");
        let model = streaming::create_and_emit(&ctx.db, &pathway, None)
            .await
            .expect("create_and_emit under outbox transport");

        let sink = FluvioSink::connect(&endpoint, "mxi.care_pathway.events.test")
            .await
            .expect("connect FluvioSink to the live broker");

        let published = relay::drain_once(&ctx.db, &sink, 10)
            .await
            .expect("drain_once against the real sink");
        assert!(published >= 1, "at least this row's send should succeed");

        let row = event_outbox::Model::for_entity(&ctx.db, model.pid)
            .await
            .expect("load the outbox row")
            .into_iter()
            .next()
            .expect("the row exists");
        assert!(
            row.published_at.is_some(),
            "a successful send to the real broker must stamp published_at"
        );
    })
    .await;
}
