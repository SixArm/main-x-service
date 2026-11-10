//! Persistence contracts the request-level journeys do not reach:
//! `updated_at` moving on an update (PF-U1), the same-day `Red2Green`
//! upsert under two writers (PF-R1), and the outbox relay poll's
//! `FOR UPDATE SKIP LOCKED` (PF-O1).

use axa_test::prelude::*;
use chrono::{DateTime, FixedOffset};
use patient_flow_service::app::App;
use patient_flow_service::models::event_outbox::{self, OutboxInsert};
use patient_flow_service::models::red_green_days;
use patient_flow_service::streaming::{Envelope, SCHEMA_VERSION};
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

use super::seed_ward;

/// A pool of its own on the test database, for a test that holds two
/// transactions at once: the app's own test pool may have just one
/// connection, so a second `begin()` on it would wait forever.
async fn two_connection_pool(db: &sqlx::PgPool) -> sqlx::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with((*db.connect_options()).clone())
        .await
        .expect("connect a two-connection pool")
}

fn stamp(row: &Value, field: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(row[field].as_str().expect(field)).expect(field)
}

/// `updated_at` moved past `created_at`.
fn assert_moved(row: &Value, what: &str) {
    assert!(
        stamp(row, "updated_at") > stamp(row, "created_at"),
        "{what}: updated_at must move forward on an update, got {row}"
    );
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL via DATABASE_URL; run with `cargo test -- --ignored`"]
// PF-U1: an UPDATE stamps `updated_at`, so it no longer always equals
// `created_at`. One update per kind of statement: a COALESCE edit
// (ward, bay) and a state transition (bed).
async fn an_update_moves_updated_at_forward() {
    request::<App, _, _>(|request, _ctx| async move {
        let (ward_pid, bed_pids) = seed_ward!(&request, "inpatient", None, 1).await;
        let ward: Value = request
            .put(&format!("/api/wards/{ward_pid}"))
            .json(&json!({ "name": "Renamed Ward" }))
            .await
            .json();
        assert_eq!(ward["name"], "Renamed Ward");
        assert_moved(&ward, "ward update");

        let bed: Value = request
            .post(&format!("/api/beds/{}/state", bed_pids[0]))
            .json(&json!({ "transition": "close", "reason": "maintenance" }))
            .await
            .json();
        assert_eq!(bed["state"], "closed");
        assert_moved(&bed, "bed transition");

        let bay_pid = bed["bay_pid"].as_str().expect("bay pid");
        let bay: Value = request
            .put(&format!("/api/bays/{bay_pid}"))
            .json(&json!({ "closed_to_admissions": true }))
            .await
            .json();
        assert_eq!(bay["closed_to_admissions"], true);
        assert_moved(&bay, "bay update");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL via DATABASE_URL; run with `cargo test -- --ignored`"]
// PF-R1: two writers for one stay and day give one row and no error —
// the second updates the day instead of tripping `red_green_stay_day`.
// Deterministic rather than timing-based: both transactions are open
// before either writes, which is the window two concurrent POSTs share
// (each saw no row for today), and they commit in a fixed order.
async fn two_same_day_red_green_writes_give_one_row() {
    request::<App, _, _>(|_request, ctx| async move {
        let pool = two_connection_pool(&ctx.db).await;
        let stay_pid = Uuid::new_v4();
        let today = chrono::Utc::now().date_naive();

        let mut first = pool.begin().await.expect("begin first");
        let mut second = pool.begin().await.expect("begin second");
        red_green_days::Model::upsert(&mut *first, stay_pid, today, "red", json!([]), None)
            .await
            .expect("first write");
        first.commit().await.expect("commit first");
        let row = red_green_days::Model::upsert(
            &mut *second,
            stay_pid,
            today,
            "green",
            json!(["awaiting_social_care"]),
            Some("second writer"),
        )
        .await
        .expect("the second same-day write updates the day, not 409");
        second.commit().await.expect("commit second");

        assert_eq!(row.classification, "green");
        let journal = red_green_days::Model::for_stay(&pool, stay_pid)
            .await
            .expect("journal");
        assert_eq!(journal.len(), 1, "one row per stay per day");
        assert_eq!(journal[0].classification, "green", "the later write wins");
        assert_eq!(journal[0].note.as_deref(), Some("second writer"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL via DATABASE_URL; run with `cargo test -- --ignored`"]
// PF-O1: rows one relay holds are invisible to a second relay polling at
// the same time, so two relays never ship one event twice (SEC-B6).
async fn a_second_relay_does_not_see_rows_the_first_holds() {
    request::<App, _, _>(|_request, ctx| async move {
        let pool = two_connection_pool(&ctx.db).await;
        let env = Envelope {
            event_id: Uuid::new_v4(),
            schema_version: SCHEMA_VERSION,
            entity: "stay".to_string(),
            kind: "stay_admitted".to_string(),
            pid: Uuid::new_v4().to_string(),
            seq: 1,
            actor: None,
            name: "Relay Poll Lock Fixture".to_string(),
            data: None,
        };
        let inserted = OutboxInsert::from_envelope(&env, chrono::Utc::now().into())
            .expect("envelope")
            .insert_on(&pool)
            .await
            .expect("insert outbox row");

        // Relay one claims every unpublished row, and keeps them locked.
        let mut first = pool.begin().await.expect("begin first");
        let claimed = event_outbox::Model::unpublished(&mut *first, 10_000)
            .await
            .expect("first poll");
        assert!(
            claimed.iter().any(|r| r.id == inserted.id),
            "relay one claims the row"
        );

        // Relay two, polling meanwhile, must not see that row.
        let mut second = pool.begin().await.expect("begin second");
        let seen = event_outbox::Model::unpublished(&mut *second, 10_000)
            .await
            .expect("second poll");
        assert!(
            seen.iter().all(|r| r.id != inserted.id),
            "a row another relay holds is skipped"
        );
        second.rollback().await.expect("rollback second");

        // Relay one acks; the row leaves the unpublished set.
        let acked = event_outbox::Model::mark_published(&mut *first, &[inserted.id])
            .await
            .expect("ack");
        assert_eq!(acked, 1);
        first.commit().await.expect("commit first");
        let after = event_outbox::Model::unpublished(&pool, 10_000)
            .await
            .expect("poll after ack");
        assert!(
            after.iter().all(|r| r.id != inserted.id),
            "an acked row is not polled again"
        );
    })
    .await;
}
