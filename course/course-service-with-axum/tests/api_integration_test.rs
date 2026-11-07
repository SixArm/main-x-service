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

/// Health probe returns `200` with the `healthy` body.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn health_returns_ok() {
    let app = common::create_test_router().await;
    let (status, body) = send(&app, Method::GET, "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "course-service");
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
    let created: Course = serde_json::from_value(env.clone()).unwrap();
    assert_ne!(
        created.id.to_string(),
        "00000000-0000-0000-0000-000000000000"
    );
    let id = created.id;

    // Get
    let (status, env) = send(&app, Method::GET, &format!("/api/courses/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let fetched: Course = serde_json::from_value(env.clone()).unwrap();
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
    assert_eq!(env["course_code"], "EDIT202");

    // Soft delete
    let (status, _) = send(&app, Method::DELETE, &format!("/api/courses/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Subsequent GET → 404
    let (status, _) = send(&app, Method::GET, &format!("/api/courses/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Invalid create body yields a `422` problem with a field-scoped `errors` array.
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
    assert_eq!(env["code"], "validation");
    let details = &env["errors"];
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

    // The unique token leads the generated name (`<token> Search`), so
    // querying it matches this record and no other test's leftovers.
    let token = suffix.split_whitespace().next().expect("unique token");

    let uri = format!("/api/courses/search?q={token}");
    let (status, env) = send(&app, Method::GET, &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    let items = env["items"].as_array().expect("items array");
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
    let created: Course = serde_json::from_value(env.clone()).unwrap();

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
    let hits = env.as_array().expect("array of ScoredCandidate");
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
    let created: Course = serde_json::from_value(env.clone()).unwrap();

    let probe = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": created.name,
    });
    let (status, env) = send(&app, Method::POST, "/api/courses/match", Some(probe)).await;
    assert_eq!(status, StatusCode::OK);
    let arr = env.as_array().expect("array");
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
    let main_id = ea["id"].as_str().unwrap().to_string();
    let dup_id = eb["id"].as_str().unwrap().to_string();

    let req = json!({
        "main_course_id": main_id,
        "duplicate_course_id": dup_id,
        "merge_reason": "Integration test",
        "merged_by": "test-suite",
    });
    let (status, env) = send(&app, Method::POST, "/api/courses/merge", Some(req)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(env["merge_record"]["main_course_id"], json!(main_id));
    assert_eq!(env["merge_record"]["duplicate_course_id"], json!(dup_id));
    assert_eq!(env["merge_record"]["status"], "Completed");

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
    let data = &env;
    assert!(data["courses_scanned"].is_number());
    assert!(data["duplicates_found"].is_number());
    assert!(data["auto_merged"].is_number());
    assert!(data["queued_for_review"].is_number());
    assert!(data["review_items"].is_array());
}

/// T-27 — a batch-dedup run's candidates are persisted to
/// `course_match_scores`: they survive as a fresh repository read (not
/// just this run's own response body), a `Pending` item can be decided
/// exactly once (first-writer-wins), and a re-scan upserts — refreshing
/// the stored score while leaving a decided row's `status` untouched.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored"]
async fn review_queue_persists_across_a_rescan_and_decides_once() {
    let (app, state) = common::boot().await;

    // Two near-duplicate courses, seeded directly through the repository
    // (bypassing the create endpoint's real-time duplicate check — see
    // `common::seed_course`'s doc comment). A shared token keeps them
    // discoverable as a candidate pair via the fuzzy name search; the
    // differing suffix keeps their match score below 1.0, so the pair
    // lands in the review queue rather than auto-merging.
    let token = uuid::Uuid::new_v4().simple().to_string();
    let name_a = format!("{token} ReviewQueuePersist Alpha");
    let name_b = format!("{token} ReviewQueuePersist Beta");
    let a = common::seed_course(&state, &name_a).await;
    let b = common::seed_course(&state, &name_b).await;

    // Run the batch scan. A wide gap between the two thresholds keeps
    // this robust to the exact matcher score: well above `threshold`
    // (found, queued) and comfortably below `auto_merge_threshold`
    // (never auto-merged away before this test can inspect it).
    let dedup_req = json!({
        "threshold": 0.50,
        "max_candidates": 10,
        "auto_merge_threshold": 0.999,
    });
    let (status, env) = send(
        &app,
        Method::POST,
        "/api/courses/deduplicate",
        Some(dedup_req.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {env}");
    let review_items = env["review_items"].as_array().expect("review_items array");
    let seeded_pair = review_items
        .iter()
        .find(|i| {
            let ids = [
                i["course_id_a"].as_str().unwrap_or_default(),
                i["course_id_b"].as_str().unwrap_or_default(),
            ];
            ids.contains(&a.to_string().as_str()) && ids.contains(&b.to_string().as_str())
        })
        .unwrap_or_else(|| panic!("seeded pair not found among review_items: {review_items:?}"));
    assert_eq!(seeded_pair["status"], "Pending");
    let item_id = seeded_pair["id"].as_str().unwrap().to_string();

    // Survives "restart": queryable via a fresh repository read, not just
    // this run's own response body.
    let (status, env) = send(&app, Method::GET, "/api/courses/review-queue", None).await;
    assert_eq!(status, StatusCode::OK);
    let items = env["items"].as_array().expect("items array");
    assert!(
        items.iter().any(|i| i["id"] == json!(item_id)),
        "persisted item missing from GET /review-queue: {items:?}"
    );

    // Decide it — confirmed.
    let (status, env) = send(
        &app,
        Method::POST,
        &format!("/api/courses/review-queue/{item_id}/decision"),
        Some(json!({ "status": "Confirmed", "reviewed_by": "test-suite" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {env}");
    assert_eq!(env["status"], "Confirmed");
    assert_eq!(env["reviewed_by"], "test-suite");

    // Deciding an already-decided item is a 422 (first-writer-wins).
    let (status, _) = send(
        &app,
        Method::POST,
        &format!("/api/courses/review-queue/{item_id}/decision"),
        Some(json!({ "status": "Rejected" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Deciding an unknown id is a 404.
    let (status, _) = send(
        &app,
        Method::POST,
        "/api/courses/review-queue/00000000-0000-0000-0000-000000000000/decision",
        Some(json!({ "status": "Confirmed" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // A re-scan upserts: the score columns refresh, but the decided
    // row's `status` is left untouched.
    let (status, env) = send(
        &app,
        Method::POST,
        "/api/courses/deduplicate",
        Some(dedup_req),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {env}");
    let (status, env) = send(
        &app,
        Method::GET,
        "/api/courses/review-queue?status=Confirmed",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = env["items"].as_array().expect("items array");
    assert!(
        items.iter().any(|i| i["id"] == json!(item_id)),
        "decided row must keep its Confirmed status across a re-scan: {items:?}"
    );
}

/// `CourseInstance` sub-resource: create → list → soft-delete round trip.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn instance_subresource_round_trips() {
    let app = common::create_test_router().await;
    let course_body = common::course_json("InstanceParent");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(course_body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let course_id = env["id"].as_str().unwrap().to_string();

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
    let inst_id = env["id"].as_str().unwrap().to_string();

    // List
    let (status, env) = send(
        &app,
        Method::GET,
        &format!("/api/courses/{course_id}/instances"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = env.as_array().unwrap();
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
    let course_id = env["id"].as_str().unwrap().to_string();

    // Update so we have two entries
    let mut update_body = env.clone();
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
    let entries = env.as_array().unwrap();
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
    let course_id = env["id"].as_str().unwrap().to_string();

    let (status, env) = send(
        &app,
        Method::GET,
        &format!("/api/courses/{course_id}/masked"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(env["provider_id"].is_null());
}

/// The GDPR export wraps the record in the source/schema envelope.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn gdpr_export_envelopes_the_record() {
    let app = common::create_test_router().await;
    let body = common::course_json("Export");
    let (status, env) = send(&app, Method::POST, "/api/courses", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED);
    let course_id = env["id"].as_str().unwrap().to_string();

    let (status, env) = send(
        &app,
        Method::GET,
        &format!("/api/courses/{course_id}/export"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let env_obj = &env;
    assert_eq!(env_obj["source"], "course-service");
    assert_eq!(env_obj["schema"], "https://schema.org/Course");
    assert!(env_obj["course"]["id"].is_string());
    assert!(env_obj["exported_at"].is_string());
}

/// Whether `GET /api/records/verify` names the record `id` as mismatched.
async fn is_mismatched(app: &axum::Router, id: &str) -> bool {
    let (status, report) = send(app, Method::GET, "/api/records/verify?limit=1000", None).await;
    assert_eq!(status, StatusCode::OK, "verify failed: {report}");
    report["mismatched"]
        .as_array()
        .expect("a mismatched list")
        .iter()
        .any(|m| m["id"] == id)
}

/// A record verifies the moment it is written, and again after an update,
/// while a row edited in SQL is still caught (tasks.md INT-V1, the defect
/// event's EVT-V1 found). The digests used to cover the course the caller
/// sent, so a timestamp Postgres rounds to microseconds flagged every fresh
/// record. Child rows (identifiers, links, text values) are included,
/// since the pre-image covers them and the database decides their order
/// on read-back.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn a_written_record_verifies() {
    let app = common::create_test_router().await;

    // A second course to link to, since `course_links` has a foreign key.
    let (status, other) = send(
        &app,
        Method::POST,
        "/api/courses",
        Some(common::course_json("VerifyLinkTarget")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create failed: {other}");
    let other_id = other["id"].as_str().expect("an id").to_string();

    let mut body = common::course_json("VerifiesWhenWritten");
    body["keywords"] = json!(["integrity", "audit"]);
    body["identifiers"] = json!([
        { "property_id": "Doi", "value": "10.1000/verify-when-written" },
        { "property_id": "LmsCourseId", "value": "lms-verify-1" },
    ]);
    body["links"] = json!([
        { "other_course_id": other_id, "link_type": "seealso" },
        { "other_course_id": other_id, "link_type": "prerequisite" },
    ]);
    body["created_at"] = json!("2001-01-01T00:00:00.987654321Z");
    body["updated_at"] = json!("2001-01-01T00:00:00.987654321Z");
    let (status, created) = send(&app, Method::POST, "/api/courses", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "create failed: {created}");
    let id = created["id"].as_str().expect("an id").to_string();
    assert!(
        !is_mismatched(&app, &id).await,
        "a fresh record must verify"
    );

    let mut changed = created.clone();
    changed["description"] = json!("Edited after creation");
    let (status, updated) = send(
        &app,
        Method::PUT,
        &format!("/api/courses/{id}"),
        Some(changed),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "update failed: {updated}");
    assert!(
        !is_mismatched(&app, &id).await,
        "an updated record must verify"
    );

    // And verification still catches a change made behind the service's
    // back, so the two checks above cannot pass by verifying nothing.
    let pool = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL"))
        .await
        .expect("connect for the tamper step");
    sqlx::query("UPDATE courses SET name = name || ' (edited in SQL)' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .expect("tamper with the row");
    assert!(
        is_mismatched(&app, &id).await,
        "a row edited in SQL must be flagged"
    );
}
