#![warn(clippy::pedantic)]

//! `EVENT_EVENT_TRANSPORT=outbox` reaches the REST write path (tasks.md
//! EVT-T1). The REST repository used to be built without the configured
//! transport, so it stayed on the memory one and a REST create never wrote
//! its `event_outbox` row, whatever the setting said.
//!
//! A test binary of its own, because the transport is read once per
//! process: setting the variable here cannot leak into another suite.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt;

/// A REST create under the outbox transport writes one `created` outbox
/// row for the new event, committed with it.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_rest_create_writes_its_outbox_row() {
    // SAFETY: DB-gated suites run with `--test-threads=1`, and this is the
    // only test in this binary, so nothing reads the environment
    // concurrently with this write.
    unsafe {
        std::env::set_var("EVENT_EVENT_TRANSPORT", "outbox");
    }
    let app = common::create_test_router().await;
    let payload = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "active": true,
        "name": common::unique_event_name("RestOutbox"),
        "start_date": "2026-06-01T18:00:00Z",
        "event_status": "scheduled",
        "event_attendance_mode": "offline",
        "event_type": "conference",
        "all_day": false,
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create failed: {}",
        String::from_utf8_lossy(&body)
    );
    let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = uuid::Uuid::parse_str(created["id"].as_str().expect("an id")).unwrap();

    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL"))
        .await
        .expect("connect to check the outbox");
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_outbox WHERE entity_pid = $1 AND kind = 'created'",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("count outbox rows");
    assert_eq!(rows, 1, "the REST create must write exactly one outbox row");
}
