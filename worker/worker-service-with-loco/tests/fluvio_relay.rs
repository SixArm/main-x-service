//! Round-trip the durable-bus relay against a **real** Fluvio broker
//! (BUS-3, ported from the case-service BUS-1 reference;
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
//! DATABASE_URL=postgres://loco:loco@127.0.0.1:5432/worker_service_test \
//!   WORKER_FLUVIO_ENDPOINT=127.0.0.1:9103 \
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
//! (the reference this file is ported from) and person's
//! `s3_round_trip_against_a_live_endpoint` (`src/bulk/store.rs`, BLK-4).
//!
//! **Deviation from the case template:** case drives this test through
//! `loco_rs::testing::prelude::request::<App, _, _>` because that crate's
//! dev-dependencies enable loco's `testing` feature. This crate's
//! dev-dependencies do not, and its own DB-gated outbox atomicity tests
//! (`src/db/repositories.rs::tests::create_enqueues_a_created_outbox_row`)
//! already establish the alternative: connect directly via `DATABASE_URL`
//! and drive the repository. This test follows that established local
//! pattern instead of introducing the loco test harness for one file.
#![cfg(feature = "fluvio")]

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use worker_service::db::models::event_outbox;
use worker_service::db::{SeaOrmWorkerRepository, WorkerRepository};
use worker_service::models::{Gender, HumanName, Worker};
use worker_service::relay::{self, FluvioSink};
use worker_service::streaming::EventTransport;

fn a_worker(family: &str) -> Worker {
    let name = HumanName {
        use_type: None,
        family: family.to_string(),
        given: vec!["FluvioRoundTrip".into()],
        prefix: vec![],
        suffix: vec![],
    };
    Worker::new(name, Gender::Unknown)
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL) AND a live Fluvio broker; see this file's module docs"]
async fn fluvio_relay_publishes_an_outbox_row_to_a_real_topic() {
    let database_url =
        std::env::var("DATABASE_URL").expect("set DATABASE_URL to a migrated test database");
    let endpoint = std::env::var("WORKER_FLUVIO_ENDPOINT")
        .expect("set WORKER_FLUVIO_ENDPOINT to the broker's SC address (e.g. 127.0.0.1:9103)");

    let db = sea_orm::Database::connect(&database_url)
        .await
        .expect("connect to DATABASE_URL");

    // Durable transport, so `create` writes an outbox row in the same
    // transaction as the worker insert.
    let repo = SeaOrmWorkerRepository::new(db.clone()).with_transport(EventTransport::Outbox);
    let worker = repo
        .create(&a_worker("FluvioRelayRoundTrip"))
        .await
        .expect("create under outbox transport");

    let sink = FluvioSink::connect(&endpoint, "mxi.worker.events.test")
        .await
        .expect("connect FluvioSink to the live broker");

    let published = relay::drain_once(&db, &sink, 10)
        .await
        .expect("drain_once against the real sink");
    assert!(published >= 1, "at least this row's send should succeed");

    let row = event_outbox::Entity::find()
        .filter(event_outbox::Column::EntityPid.eq(worker.id))
        .filter(event_outbox::Column::Kind.eq("created"))
        .one(&db)
        .await
        .expect("load the outbox row")
        .expect("the row exists");
    assert!(
        row.published_at.is_some(),
        "a successful send to the real broker must stamp published_at"
    );
}
