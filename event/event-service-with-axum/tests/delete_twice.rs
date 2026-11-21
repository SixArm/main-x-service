//! SEC-B12: deleting a event that is already deleted, or never existed, is a
//! `404`. It used to answer `204` and re-stamp `deleted_at`, losing when
//! the record was really deleted, and publish another `deleted` event.
//!
//! SEC-B13: an update does not bring a deleted record back. The update
//! wrote the deletion columns from the request (or as cleared), so a plain
//! `PUT` on a deleted record answered `200` and it was served again.
//!
//! `#[ignore]`d — run with `DB_URL=… cargo test --test delete_twice -- --ignored`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;

mod common;

async fn app() -> axum::Router {
    common::create_test_router().await
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(b) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(b).unwrap())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn deleted_at(id: uuid::Uuid) -> Option<chrono::DateTime<chrono::Utc>> {
    let url = std::env::var("DB_URL").expect("DB_URL");
    let db = sqlx::PgPool::connect(&url).await.unwrap();
    sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT deleted_at FROM events WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&db)
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn a_second_delete_is_404_and_keeps_the_first_deletion_time() {
    let app = app().await;
    let name = format!("Deletetwice {}", uuid::Uuid::new_v4().simple());
    let (status, created) = call(&app, "POST", "/api/events", Some(&json!({ "id": "00000000-0000-0000-0000-000000000000", "active": true, "name": name, "start_date": "2026-06-01T18:00:00Z", "end_date": "2026-06-01T20:00:00Z", "event_status": "scheduled", "event_attendance_mode": "offline", "event_type": "conference", "all_day": false }))).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id: uuid::Uuid = created["id"].as_str().unwrap().parse().unwrap();
    let uri = format!("/api/events/{id}");

    let (status, _) = call(&app, "DELETE", &uri, None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let first = deleted_at(id).await.expect("deleted_at is stamped");

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let (status, body) = call(&app, "DELETE", &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(
        deleted_at(id).await,
        Some(first),
        "the deletion time is not rewritten"
    );

    let (status, _) = call(
        &app,
        "DELETE",
        &format!("/api/events/{}", uuid::Uuid::new_v4()),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "an unknown id");
}

#[tokio::test]
#[ignore = "requires PostgreSQL (DB_URL)"]
async fn an_update_does_not_bring_a_deleted_record_back() {
    let app = app().await;
    let name = format!("Staydeleted {}", uuid::Uuid::new_v4().simple());
    let (status, created) = call(&app, "POST", "/api/events", Some(&json!({ "id": "00000000-0000-0000-0000-000000000000", "active": true, "name": name, "start_date": "2026-06-01T18:00:00Z", "end_date": "2026-06-01T20:00:00Z", "event_status": "scheduled", "event_attendance_mode": "offline", "event_type": "conference", "all_day": false }))).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let id: uuid::Uuid = created["id"].as_str().unwrap().parse().unwrap();
    let uri = format!("/api/events/{id}");
    assert_eq!(
        call(&app, "DELETE", &uri, None).await.0,
        StatusCode::NO_CONTENT
    );
    let first = deleted_at(id).await.expect("deleted_at is stamped");

    let name = "Brought back".to_string();
    let (status, body) = call(&app, "PUT", &uri, Some(&json!({ "id": "00000000-0000-0000-0000-000000000000", "active": true, "name": name, "start_date": "2026-06-01T18:00:00Z", "end_date": "2026-06-01T20:00:00Z", "event_status": "scheduled", "event_attendance_mode": "offline", "event_type": "conference", "all_day": false }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(
        call(&app, "GET", &uri, None).await.0,
        StatusCode::NOT_FOUND,
        "still deleted"
    );
    assert_eq!(
        deleted_at(id).await,
        Some(first),
        "and the deletion time is kept"
    );

    let (status, _) = call(&app, "PUT", &format!("/api/events/{}", uuid::Uuid::new_v4()), Some(&json!({ "id": "00000000-0000-0000-0000-000000000000", "active": true, "name": name, "start_date": "2026-06-01T18:00:00Z", "end_date": "2026-06-01T20:00:00Z", "event_status": "scheduled", "event_attendance_mode": "offline", "event_type": "conference", "all_day": false }))).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "an unknown id");
}
