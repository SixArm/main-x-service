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
//! PERSON_FLUVIO_ENDPOINT=127.0.0.1:9203 \
//!   cargo test --features fluvio --test fluvio_relay -- --ignored
//! podman compose -f compose.fluvio.yaml down -v
//! ```
//!
//! This test exists because everything else about `FluvioSink` is
//! verified only by the compiler (`cargo build --features fluvio`
//! confirms the real `fluvio` 0.50 API is used correctly, but not that a
//! `send` actually reaches a topic). Saying "the Fluvio sink is tested"
//! on the strength of the compile check alone would be false — same
//! posture as this crate's own
//! `s3_round_trip_against_a_live_endpoint` (`src/bulk/store.rs`, BLK-4),
//! and as case-service's `fluvio_relay.rs` (BUS-1), which this file was
//! ported from and which has likewise never been executed in this repo.
//!
//! Deviation from the case-service reference: case drives this round trip
//! through its loco `App` + `streaming::create_and_emit` (its layout has
//! that helper). This crate's write path enqueues the outbox row inside
//! `PersonRepository::create`/`update`/`delete`, with no equivalent
//! single-call helper, so this test instead builds the outbox row
//! directly with `db::outbox::OutboxInsert::for_event` + `insert_on` over
//! a plain connection from `tests/common::db()` — the same "ground truth"
//! pattern the tamper-evidence integration tests already use in this
//! crate — and inserts it against a fresh `Person` domain value rather
//! than a persisted row (the relay only reads `event_outbox`, so no
//! `persons` row is required to exercise it).
#![cfg(feature = "fluvio")]

mod common;

use person_service::db::models::event_outbox;
use person_service::db::outbox::OutboxInsert;
use person_service::models::{Gender, HumanName, Person};
use person_service::relay::{self, FluvioSink};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tokio::test]
#[ignore = "requires PostgreSQL AND a live Fluvio broker; see this file's module docs"]
async fn fluvio_relay_publishes_an_outbox_row_to_a_real_topic() {
    let endpoint = std::env::var("PERSON_FLUVIO_ENDPOINT")
        .expect("set PERSON_FLUVIO_ENDPOINT to the broker's SC address (e.g. 127.0.0.1:9203)");

    let db = common::db().await;

    let person = Person::new(
        HumanName {
            use_type: None,
            family: "FluvioRelay".to_string(),
            given: vec!["RoundTrip".to_string()],
            prefix: vec![],
            suffix: vec![],
        },
        Gender::Unknown,
    );

    let insert = OutboxInsert::for_event(&person, person_service::streaming::EventKind::Created)
        .expect("build the outbox row from the envelope");
    insert.insert_on(&db).await.expect("enqueue the outbox row");

    let sink = FluvioSink::connect(&endpoint, "mxi.person.events.test")
        .await
        .expect("connect FluvioSink to the live broker");

    let published = relay::drain_once(&db, &sink, 10)
        .await
        .expect("drain_once against the real sink");
    assert!(published >= 1, "at least this row's send should succeed");

    let row = event_outbox::Entity::find()
        .filter(event_outbox::Column::EntityPid.eq(person.id))
        .one(&db)
        .await
        .expect("load the outbox row")
        .expect("the row exists");
    assert!(
        row.published_at.is_some(),
        "a successful send to the real broker must stamp published_at"
    );
}
