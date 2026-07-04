#![warn(clippy::pedantic)]

//! REST API integration tests. Requires a running `PostgreSQL`
//! reachable via `DATABASE_URL`.
//!
//! All tests are marked `#[ignore = "requires a running PostgreSQL via DATABASE_URL"]` so they are skipped by a bare
//! `cargo test`; run them explicitly with
//! `DATABASE_URL=… cargo test --test api_integration_test -- --ignored`.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use event_service::{api::ApiResponse, models::Event};
use serde_json::json;
use tower::ServiceExt;

/// `GET /api/v1/health` returns 200 and names the service.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
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

/// Creating an event mints a fresh id and the record reads back
/// identically via `GET /api/v1/events/{id}`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
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
                .uri(format!("/api/v1/events/{}", event.id))
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

/// The FHIR R5 surface is a stub: `GET /fhir/Event/{id}` is routed and
/// returns `501 Not Implemented` with an `OperationOutcome` body (spec
/// §6.8) — not a `404`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn fhir_event_returns_501_not_implemented() {
    let app = common::create_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri("/fhir/Event/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        s.contains("OperationOutcome"),
        "expected OperationOutcome, got {s}"
    );
}

/// An empty `name` is rejected with `422 Unprocessable Entity`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
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
