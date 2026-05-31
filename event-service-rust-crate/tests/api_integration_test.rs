//! REST API integration tests. Requires a running PostgreSQL
//! reachable via `DATABASE_URL`.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use event_service::{api::ApiResponse, models::Event};
use serde_json::json;
use tower::ServiceExt;

#[tokio::test]
async fn health_check_returns_healthy() {
    let app = common::create_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("healthy"));
    assert!(s.contains("event-service"));
}

#[tokio::test]
async fn create_event_round_trip() {
    let app = common::create_test_router().await;
    let title = common::unique_event_name("CreateRoundTrip");

    let payload = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "active": true,
        "name": title,
        "start_date": "2026-06-01T18:00:00Z",
        "end_date": "2026-06-01T20:00:00Z",
        "event_status": "scheduled",
        "event_attendance_mode": "offline",
        "event_type": "conference",
        "all_day": false,
    });

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(create.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: ApiResponse<Event> = serde_json::from_slice(&body).unwrap();
    let event = created.data.expect("event in body");
    assert_eq!(event.name, title);
    assert_ne!(event.id.to_string(), "00000000-0000-0000-0000-000000000000");

    let get = app
        .oneshot(
            Request::builder()
                .uri(&format!("/api/v1/events/{}", event.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let body = axum::body::to_bytes(get.into_body(), usize::MAX)
        .await
        .unwrap();
    let fetched: ApiResponse<Event> = serde_json::from_slice(&body).unwrap();
    let fetched = fetched.data.expect("event in body");
    assert_eq!(fetched.id, event.id);
    assert_eq!(fetched.name, title);
}

#[tokio::test]
async fn validation_rejects_missing_name() {
    let app = common::create_test_router().await;
    let payload = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "active": true,
        "name": "",
        "start_date": "2026-06-01T18:00:00Z",
        "event_status": "scheduled",
        "event_attendance_mode": "offline",
        "event_type": "generic",
        "all_day": false,
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/events")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
