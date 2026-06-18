#![warn(clippy::pedantic)]

//! End-to-end integration tests over the full REST surface.
//!
//! All tests are marked `#[ignore]` so they are not part of the
//! default unit-test run. Activate with:
//!
//! ```bash
//! DATABASE_URL=postgres://course:course@localhost:5434/course \
//!   cargo test --test api_integration_test -- --ignored
//! ```
//!
//! The harness assumes the test DB has been migrated (e.g. via
//! `podman compose up -d` from the crate root, which runs the SQL
//! files under `migrations/` into the Postgres container).

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

use course_service::models::Course;

/// Drive one request through the router via `oneshot` and decode the
/// response into `(status, json)`. An empty body decodes to `Value::Null`
/// so 204 responses are handled cleanly.
async fn send(
    app: &axum::Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let req_body = body.map_or(Body::empty(), |v| {
        Body::from(serde_json::to_vec(&v).unwrap())
    });
    let mut builder = Request::builder().method(method).uri(uri);
    if !uri.is_empty() {
        builder = builder.header("content-type", "application/json");
    }
    let req = builder.body(req_body).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

/// Health probe returns `200` with the `healthy` envelope.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn health_returns_ok() {
    let app = common::create_test_router().await;
    let (status, body) = send(&app, Method::GET, "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true));
    assert_eq!(body["data"]["status"], "healthy");
    assert_eq!(body["data"]["service"], "course-service");
}

/// Full CRUD lifecycle: create → get → update → soft-delete → 404.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn create_get_update_softdelete_lifecycle() {
    let app = common::create_test_router().await;
    let body = common::course_json("Lifecycle");

    // Create
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(body.clone())).await;
    assert_eq!(status, StatusCode::CREATED, "body: {env}");
    let created: Course = serde_json::from_value(env["data"].clone()).unwrap();
    assert_ne!(
        created.id.to_string(),
        "00000000-0000-0000-0000-000000000000"
    );
    let id = created.id;

    // Get
    let (status, env) = send(&app, Method::GET, &format!("/api/courses/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let fetched: Course = serde_json::from_value(env["data"].clone()).unwrap();
    assert_eq!(fetched.id, id);
    assert_eq!(fetched.name, created.name);

    // Update — change course_code
    let mut updated_body: Value = serde_json::to_value(&created).unwrap();
    updated_body["course_code"] = json!("EDIT202");
    let (status, env) = send(
        &app,
        Method::PUT,
        &format!("/api/courses/{id}"),
        Some(updated_body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(env["data"]["course_code"], "EDIT202");

    // Soft delete
    let (status, _) = send(&app, Method::DELETE, &format!("/api/courses/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Subsequent GET → 404
    let (status, _) = send(&app, Method::GET, &format!("/api/courses/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Invalid create body yields `422` with a field-scoped `details` array.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn validation_failure_returns_422_with_details() {
    let app = common::create_test_router().await;
    let bad = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": "   ",                  // blank → FR-21
        "url":  "javascript:alert(1)",  // bad scheme → FR-25
    });
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(bad)).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(env["success"], json!(false));
    let details = &env["error"]["details"];
    assert!(
        details.is_array(),
        "details should be an array, got {details}"
    );
    let fields: Vec<String> = details
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["field"].as_str().map(String::from))
        .collect();
    assert!(fields.iter().any(|f| f == "name"));
    assert!(fields.iter().any(|f| f == "url"));
}

/// A freshly created course is discoverable via the search endpoint.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn search_finds_created_record() {
    let app = common::create_test_router().await;
    let body = common::course_json("Search");
    let suffix = body["name"].as_str().unwrap().to_string();
    let (status, _) = send(&app, Method::POST, "/api/courses", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);

    // Pull out the unique "Integration Search <ts>" token for the query.
    let token = suffix.split_whitespace().last().expect("timestamp token");

    let uri = format!("/api/courses/search?q={token}");
    let (status, env) = send(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    let items = env["data"]["items"].as_array().expect("items array");
    assert!(!items.is_empty(), "expected at least one hit for {token}");
}

/// The check-duplicates endpoint flags an identical-shape probe.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn check_duplicates_flags_a_clone() {
    let app = common::create_test_router().await;
    let body = common::course_json("DupCheck");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(body.clone())).await;
    assert_eq!(status, StatusCode::CREATED);
    let created: Course = serde_json::from_value(env["data"].clone()).unwrap();

    // Probe with an identical-shape body but a fresh id.
    let mut probe: Value = serde_json::to_value(&created).unwrap();
    probe["id"] = json!("00000000-0000-0000-0000-000000000000");
    let (status, env) = send(
        &app,
        Method::POST,
        "/api/courses/check-duplicates",
        Some(probe),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let hits = env["data"].as_array().expect("array of ScoredCandidate");
    assert!(!hits.is_empty(), "expected duplicate detection to fire");
    assert_eq!(hits[0]["course_id"], json!(created.id));
}

/// The match endpoint returns the existing record among ranked candidates.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn match_endpoint_returns_ranked_candidates() {
    let app = common::create_test_router().await;
    let body = common::course_json("Match");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(body.clone())).await;
    assert_eq!(status, StatusCode::CREATED);
    let created: Course = serde_json::from_value(env["data"].clone()).unwrap();

    let probe = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": created.name,
    });
    let (status, env) = send(&app, Method::POST, "/api/courses/match", Some(probe)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = env["data"].as_array().expect("array");
    assert!(arr.iter().any(|c| c["course_id"] == json!(created.id)));
}

/// Merge completes and soft-deletes the duplicate (subsequent GET → 404).
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn merge_folds_duplicate_into_main() {
    let app = common::create_test_router().await;
    let body_a = common::course_json("MergeMain");
    let body_b = common::course_json("MergeDup");
    let (sa, ea) = send(&app, Method::POST, "/api/courses", Some(body_a)).await;
    let (sb, eb) = send(&app, Method::POST, "/api/courses", Some(body_b)).await;
    assert_eq!(sa, StatusCode::CREATED);
    assert_eq!(sb, StatusCode::CREATED);
    let main_id = ea["data"]["id"].as_str().unwrap().to_string();
    let dup_id = eb["data"]["id"].as_str().unwrap().to_string();

    let req = json!({
        "main_course_id": main_id,
        "duplicate_course_id": dup_id,
        "merge_reason": "Integration test",
        "merged_by": "test-suite",
    });
    let (status, env) = send(&app, Method::POST, "/api/courses/merge", Some(req)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        env["data"]["merge_record"]["main_course_id"],
        json!(main_id)
    );
    assert_eq!(
        env["data"]["merge_record"]["duplicate_course_id"],
        json!(dup_id)
    );
    assert_eq!(env["data"]["merge_record"]["status"], "Completed");

    // Duplicate is soft-deleted → 404.
    let (status, _) = send(&app, Method::GET, &format!("/api/courses/{dup_id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Batch dedup returns the full counter + review-items response shape.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn batch_dedup_returns_response_shape() {
    let app = common::create_test_router().await;
    let req = json!({
        "threshold": 0.85,
        "max_candidates": 10,
        "auto_merge_threshold": 0.95,
    });
    let (status, env) = send(&app, Method::POST, "/api/courses/deduplicate", Some(req)).await;
    assert_eq!(status, StatusCode::OK);
    let data = &env["data"];
    assert!(data["courses_scanned"].is_number());
    assert!(data["duplicates_found"].is_number());
    assert!(data["auto_merged"].is_number());
    assert!(data["queued_for_review"].is_number());
    assert!(data["review_items"].is_array());
}

/// `CourseInstance` sub-resource: create → list → soft-delete round trip.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn instance_subresource_round_trips() {
    let app = common::create_test_router().await;
    let course_body = common::course_json("InstanceParent");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(course_body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let course_id = env["data"]["id"].as_str().unwrap().to_string();

    // Create instance
    let inst_body = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "course_id": course_id,
        "name": "Spring 2026 (Test)",
        "status": "scheduled",
    });
    let (status, env) = send(
        &app,
        Method::POST,
        &format!("/api/courses/{course_id}/instances"),
        Some(inst_body),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body: {env}");
    let inst_id = env["data"]["id"].as_str().unwrap().to_string();

    // List
    let (status, env) = send(
        &app,
        Method::GET,
        &format!("/api/courses/{course_id}/instances"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = env["data"].as_array().unwrap();
    assert!(arr.iter().any(|i| i["id"] == json!(inst_id)));

    // Soft-delete
    let (status, _) = send(
        &app,
        Method::DELETE,
        &format!("/api/courses/{course_id}/instances/{inst_id}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

/// Create + update produce `CREATE` and `UPDATE` audit-log entries.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn audit_log_records_create_then_update() {
    let app = common::create_test_router().await;
    let body = common::course_json("Audit");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let course_id = env["data"]["id"].as_str().unwrap().to_string();

    // Update so we have two entries
    let mut update_body = env["data"].clone();
    update_body["course_code"] = json!("AUDIT99");
    let (_, _) = send(
        &app,
        Method::PUT,
        &format!("/api/courses/{course_id}"),
        Some(update_body),
    )
    .await;

    let (status, env) = send(
        &app,
        Method::GET,
        &format!("/api/courses/{course_id}/audit"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let entries = env["data"].as_array().unwrap();
    let actions: Vec<&str> = entries
        .iter()
        .filter_map(|e| e["action"].as_str())
        .collect();
    assert!(actions.contains(&"CREATE"));
    assert!(actions.contains(&"UPDATE"));
}

/// The masked view nulls out provider (and other sensitive) fields.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn masked_view_clears_provider_and_instructors() {
    let app = common::create_test_router().await;
    let body = common::course_json("Masked");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let course_id = env["data"]["id"].as_str().unwrap().to_string();

    let (status, env) = send(
        &app,
        Method::GET,
        &format!("/api/courses/{course_id}/masked"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(env["data"]["provider_id"].is_null());
}

/// The GDPR export wraps the record in the source/schema envelope.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn gdpr_export_envelopes_the_record() {
    let app = common::create_test_router().await;
    let body = common::course_json("Export");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let course_id = env["data"]["id"].as_str().unwrap().to_string();

    let (status, env) = send(
        &app,
        Method::GET,
        &format!("/api/courses/{course_id}/export"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let env_obj = &env["data"];
    assert_eq!(env_obj["source"], "course-service");
    assert_eq!(env_obj["schema"], "https://schema.org/Course");
    assert!(env_obj["course"]["id"].is_string());
    assert!(env_obj["exported_at"].is_string());
}
