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
//! DATABASE_URL=postgres://course:course@localhost:5434/course \
//!   COURSE_FLUVIO_ENDPOINT=127.0.0.1:9103 \
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
//! case's `fluvio_relay_publishes_an_outbox_row_to_a_real_topic`
//! (`case/case-service-with-loco/tests/fluvio_relay.rs`, BUS-1).
//!
//! This crate has no loco `request::<App, _, _>` test harness (course's
//! integration suite drives the router directly via `tower::oneshot`
//! against an env-configured `DATABASE_URL`, see `tests/common/mod.rs`
//! and `src/db/mod.rs`'s `outbox_atomicity_tests`), so this test follows
//! that same shape rather than case's loco-request pattern: connect via
//! `DATABASE_URL`, drive the entity write through `SeaOrmCourseRepository`
//! directly (which enqueues the outbox row in the same transaction under
//! the outbox transport), then drain to a real `FluvioSink`.
#![cfg(feature = "fluvio")]

use course_service::db::models::course_outbox;
use course_service::db::{CourseRepository, SeaOrmCourseRepository};
use course_service::models::Course;
use course_service::relay::{self, FluvioSink};
use course_service::streaming::EventTransport;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

#[tokio::test]
#[ignore = "requires PostgreSQL AND a live Fluvio broker; see this file's module docs"]
async fn fluvio_relay_publishes_an_outbox_row_to_a_real_topic() {
    let database_url = std::env::var("DATABASE_URL")
        .expect("set DATABASE_URL to a running, migrated Postgres for this test");
    let endpoint = std::env::var("COURSE_FLUVIO_ENDPOINT")
        .expect("set COURSE_FLUVIO_ENDPOINT to the broker's SC address (e.g. 127.0.0.1:9103)");

    let db = sea_orm::Database::connect(&database_url)
        .await
        .expect("connect to DATABASE_URL");

    // Durable transport, so `create` writes an outbox row in the same
    // transaction as the course insert.
    let repo = SeaOrmCourseRepository::new(db.clone()).with_transport(EventTransport::Outbox);
    let course = repo
        .create(&Course::new("Fluvio Relay Round Trip"))
        .await
        .expect("create under outbox transport");

    let sink = FluvioSink::connect(&endpoint, "mxi.course.events.test")
        .await
        .expect("connect FluvioSink to the live broker");

    let published = relay::drain_once(&db, &sink, 10)
        .await
        .expect("drain_once against the real sink");
    assert!(published >= 1, "at least this row's send should succeed");

    let row = course_outbox::Entity::find()
        .filter(course_outbox::Column::EntityPid.eq(course.id))
        .one(&db)
        .await
        .expect("load the outbox row")
        .expect("the row exists");
    assert!(
        row.published_at.is_some(),
        "a successful send to the real broker must stamp published_at"
    );
}
