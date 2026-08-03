//! Round-trip the durable-bus relay against a **real** Fluvio broker
//! (BUS-3, porting BUS-1's case-service reference;
//! `agents/share/event-bus.md` §10 "Fluvio-gated" tests).
//!
//! The whole file is gated on the `fluvio` cargo feature (`#![cfg(...)]`)
//! so a default build — and the family CI, which does not pass
//! `--features fluvio` to this crate today — compiles a file with zero
//! tests rather than skipping a feature-locked one. On top of that, the
//! single test is `#[ignore]`d: it needs both `PostgreSQL` and a live
//! Fluvio broker, neither of which any automated run in this repo stands
//! up today. Run it against `compose.fluvio.yaml`:
//!
//! ```sh
//! podman compose -f compose.fluvio.yaml up -d
//! DATABASE_URL=postgres://loco:loco@localhost:5432/place_service_test \
//!   PLACE_FLUVIO_ENDPOINT=127.0.0.1:9103 \
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
//! (`person/person-service-with-loco/src/bulk/store.rs`, BLK-4).
#![cfg(feature = "fluvio")]

use place_service::db::models::event_outbox;
use place_service::db::{PlaceRepository, SeaOrmPlaceRepository};
use place_service::models::place::Place;
use place_service::relay::{self, FluvioSink};
use place_service::streaming::envelope::EventTransport;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tokio::test]
#[ignore = "requires PostgreSQL AND a live Fluvio broker; see this file's module docs"]
async fn fluvio_relay_publishes_an_outbox_row_to_a_real_topic() {
    let db_url = std::env::var("DATABASE_URL").expect("set DATABASE_URL to a migrated Postgres");
    let endpoint = std::env::var("PLACE_FLUVIO_ENDPOINT")
        .expect("set PLACE_FLUVIO_ENDPOINT to the broker's SC address (e.g. 127.0.0.1:9103)");

    let db = sea_orm::Database::connect(&db_url)
        .await
        .expect("connect to DATABASE_URL");

    // Durable transport, so `create` writes an outbox row.
    let repo = SeaOrmPlaceRepository::new(db.clone()).with_transport(EventTransport::Outbox);
    let place = repo
        .create(&Place::new("Fluvio Relay Round Trip"))
        .await
        .expect("create under outbox transport");

    let sink = FluvioSink::connect(&endpoint, "mxi.place.events.test")
        .await
        .expect("connect FluvioSink to the live broker");

    let published = relay::drain_once(&db, &sink, 10)
        .await
        .expect("drain_once against the real sink");
    assert!(published >= 1, "at least this row's send should succeed");

    let row = event_outbox::Entity::find()
        .filter(event_outbox::Column::EntityPid.eq(place.id))
        .one(&db)
        .await
        .expect("load the outbox row")
        .expect("the row exists");
    assert!(
        row.published_at.is_some(),
        "a successful send to the real broker must stamp published_at"
    );
}
