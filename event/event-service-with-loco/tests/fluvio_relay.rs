//! Round-trip the durable-bus relay against a **real** Fluvio broker
//! (BUS-3, ported from case-service's BUS-1 reference;
//! `agents/share/event-bus.md` §10 "Fluvio-gated" tests).
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
//! EVENT_FLUVIO_ENDPOINT=127.0.0.1:9203 \
//!   cargo test --features fluvio --test fluvio_relay -- --ignored
//! podman compose -f compose.fluvio.yaml down -v
//! ```
//!
//! This test exists because everything else about `FluvioSink` is
//! verified only by the compiler (`cargo build --features fluvio`
//! confirms the real `fluvio` 0.50 API is used correctly, but not that a
//! `send` actually reaches a topic). Saying "the Fluvio sink is tested"
//! on the strength of the compile check alone would be false — same
//! posture as person's `s3_round_trip_against_a_live_endpoint`
//! (`person/person-service-with-loco/src/bulk/store.rs`, BLK-4) and
//! case-service's `fluvio_relay_publishes_an_outbox_row_to_a_real_topic`
//! (`case/case-service-with-loco/tests/fluvio_relay.rs`, BUS-1).
//!
//! Unlike case-service's reference (a loco `request::<App, _, _>` test),
//! event-service's own test harness has no loco request helper — this
//! crate keeps the older hand-rolled `AppState`/repository layout (see
//! `agents/share/architecture.md` "person-style"). So this test drives
//! [`SeaOrmEventRepository`] directly with
//! [`EventTransport::Outbox`](event_service::streaming::EventTransport)
//! rather than mutating a process-wide `EVENT_EVENT_TRANSPORT` env var —
//! functionally equivalent (an outbox row is enqueued in the same
//! transaction as the create), and it avoids the `serial_test` +
//! `unsafe { set_var }` dance case's version needs.
#![cfg(feature = "fluvio")]

use chrono::{Duration as ChronoDuration, Utc};
use event_service::config::Config;
use event_service::db::models::event_outbox;
use event_service::db::{self, EventRepository, SeaOrmEventRepository};
use event_service::models::Event;
use event_service::relay::{self, FluvioSink};
use event_service::streaming::EventTransport;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tokio::test]
#[ignore = "requires PostgreSQL AND a live Fluvio broker; see this file's module docs"]
async fn fluvio_relay_publishes_an_outbox_row_to_a_real_topic() {
    let endpoint = std::env::var("EVENT_FLUVIO_ENDPOINT")
        .expect("set EVENT_FLUVIO_ENDPOINT to the broker's SC address (e.g. 127.0.0.1:9203)");

    let config = Config::from_env().expect("failed to load test config");
    let db = db::create_connection(&config.database)
        .await
        .expect("failed to connect to database");

    // Outbox transport on this repository instance only — no process-wide
    // env mutation needed (see the module docs' deviation note).
    let repo = SeaOrmEventRepository::new(db.clone()).with_transport(EventTransport::Outbox);

    let start = Utc::now() + ChronoDuration::days(1);
    let event = Event::new("Fluvio Relay Round Trip", start);
    let created = repo
        .create(&event)
        .await
        .expect("create under outbox transport");

    let sink = FluvioSink::connect(&endpoint, "mxi.event.events.test")
        .await
        .expect("connect FluvioSink to the live broker");

    let published = relay::drain_once(&db, &sink, 10)
        .await
        .expect("drain_once against the real sink");
    assert!(published >= 1, "at least this row's send should succeed");

    let row = event_outbox::Entity::find()
        .filter(event_outbox::Column::EntityPid.eq(created.id))
        .one(&db)
        .await
        .expect("load the outbox row")
        .expect("the row exists");
    assert!(
        row.published_at.is_some(),
        "a successful send to the real broker must stamp published_at"
    );
}
