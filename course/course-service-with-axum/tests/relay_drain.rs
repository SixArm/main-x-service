//! The relay's drain and purge against a real outbox, with no broker:
//! a sink that fails leaves the row unpublished, a sink that succeeds
//! stamps `published_at` and does not resend, and the retention purge
//! removes published rows. `fluvio_relay.rs` covers a live broker behind
//! the `fluvio` feature; this needs none.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test relay_drain -- --ignored`.

use course_service::config::Config;
use course_service::db::create_connection;
use course_service::db::outbox::OutboxInsert;
use course_service::models::Course;
use course_service::relay::{EventSink, SinkError, drain_once, purge_published};
use course_service::streaming::envelope::EventKind;
use std::sync::Mutex;

struct Capturing(Mutex<Vec<String>>);

#[async_trait::async_trait]
impl EventSink for Capturing {
    async fn send(
        &self,
        _entity: &str,
        key: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), SinkError> {
        self.0.lock().unwrap().push(key.to_string());
        Ok(())
    }
}

struct Failing;

#[async_trait::async_trait]
impl EventSink for Failing {
    async fn send(
        &self,
        _entity: &str,
        _key: &str,
        _payload: &serde_json::Value,
    ) -> Result<(), SinkError> {
        Err("broker unavailable".into())
    }
}

async fn published_at(db: &sqlx::PgPool, id: i64) -> Option<Option<chrono::DateTime<chrono::Utc>>> {
    sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT published_at FROM course_outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn drain_publishes_on_success_holds_on_failure_and_purge_removes() {
    let config = Config::from_env().expect("config");
    let db = create_connection(&config.database).await.expect("connect");
    let course = Course::new("Relay drain course");
    let mut conn = db.acquire().await.unwrap();
    let row = OutboxInsert::for_event(&course, EventKind::Created)
        .unwrap()
        .insert_on(&mut conn)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(
        published_at(&db, row.id).await,
        Some(None),
        "starts unpublished"
    );

    assert_eq!(
        drain_once(&db, &Failing, 10_000).await.unwrap(),
        0,
        "a failing first send publishes nothing"
    );
    assert_eq!(
        published_at(&db, row.id).await,
        Some(None),
        "the row waits for the next tick"
    );

    let sink = Capturing(Mutex::new(Vec::new()));
    assert!(drain_once(&db, &sink, 10_000).await.unwrap() >= 1);
    assert!(
        sink.0.lock().unwrap().contains(&course.id.to_string()),
        "keyed by the course id"
    );
    assert!(
        published_at(&db, row.id).await.unwrap().is_some(),
        "published_at is stamped"
    );
    assert_eq!(
        drain_once(&db, &sink, 10_000).await.unwrap(),
        0,
        "a published row is not resent"
    );

    assert!(purge_published(&db, 0).await.unwrap() >= 1);
    assert_eq!(
        published_at(&db, row.id).await,
        None,
        "retention removes published rows"
    );
}
