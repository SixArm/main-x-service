//! HTTP tests for REST handlers whose database layer is tested elsewhere
//! but whose endpoints were not: the review queue, the audit checkpoint,
//! a merge that carries the duplicate's data across, the audit listings,
//! and the `404` every single-worker endpoint but erasure gives an
//! unknown id.
//! DB-gated.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

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

/// A unique, short, letters-only family name (the index splits digits and
/// drops long tokens).
fn family(prefix: &str) -> String {
    let letters: String = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .map(|c| char::from(b'a' + u8::try_from(c.to_digit(16).unwrap()).unwrap()))
        .collect();
    format!("{prefix}{letters}")
}

async fn create(app: &axum::Router, body: &Value) -> Uuid {
    let (status, created) = call(app, "POST", "/api/workers", Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    created["id"].as_str().unwrap().parse().unwrap()
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn review_queue_lists_and_decides_once() {
    use worker_service::db::review_queue::{self, NewReviewItem};
    let app = common::create_test_router().await;
    let db = common::db().await;
    let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
    let rows = review_queue::upsert(
        &db,
        &[NewReviewItem {
            record_id_a: a,
            record_id_b: b,
            match_score: 0.83,
            match_quality: "probable".to_string(),
            detection_method: "batch_deduplication".to_string(),
            score_breakdown: None,
            evidence: None,
            status: "pending".to_string(),
        }],
    )
    .await
    .unwrap();
    let id = rows[0].id;

    let (status, listed) = call(&app, "GET", "/api/workers/review-queue", None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert!(listed.to_string().contains(&id.to_string()), "{listed}");

    let decision = json!({ "status": "confirmed" });
    let uri = format!("/api/workers/review-queue/{id}/decision");
    let (status, decided) = call(&app, "POST", &uri, Some(&decision)).await;
    assert_eq!(status, StatusCode::OK, "{decided}");
    assert_eq!(decided["status"], "confirmed");

    let (status, _) = call(&app, "POST", &uri, Some(&json!({ "status": "rejected" }))).await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "first writer wins"
    );
    let unknown = format!("/api/workers/review-queue/{}/decision", Uuid::new_v4());
    let (status, _) = call(&app, "POST", &unknown, Some(&decision)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    sqlx::query("DELETE FROM review_queue WHERE id = $1")
        .bind(id)
        .execute(&db)
        .await
        .unwrap();
}

/// A checkpoint the chain honours, and each verdict an altered one gets.
/// The chain itself is never touched: these suites share one database and
/// later tests verify the whole chain.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn audit_checkpoint_is_honoured_and_alterations_are_named() {
    let app = common::create_test_router().await;
    let id = create(
        &app,
        &json!({ "name": { "family": family("Ckpt"), "given": ["A"] }, "gender": "unknown" }),
    )
    .await;

    let (status, checkpoint) = call(&app, "GET", "/api/audit/checkpoint", None).await;
    assert_eq!(status, StatusCode::OK, "{checkpoint}");
    let verify = |c: &Value| {
        let c = c.clone();
        let app = app.clone();
        async move { call(&app, "POST", "/api/audit/checkpoint/verify", Some(&c)).await }
    };
    let (status, report) = verify(&checkpoint).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["honoured"], true, "{report}");

    // These alterations would fail the checkpoint's own MAC when a key is
    // configured; without one the chain comparison names each.
    if checkpoint["mac"].is_null() {
        let mut more_rows = checkpoint.clone();
        more_rows["rows_at_or_before"] =
            json!(checkpoint["rows_at_or_before"].as_u64().unwrap() + 1);
        assert_eq!(
            verify(&more_rows).await.1["verdict"]["verdict"],
            "rows_deleted"
        );

        let mut other_head = checkpoint.clone();
        other_head["head"] = json!("0".repeat(64));
        assert_eq!(
            verify(&other_head).await.1["verdict"]["verdict"],
            "head_changed"
        );

        let mut gone = checkpoint.clone();
        gone["anchor_id"] = json!(i64::MAX);
        assert_eq!(
            verify(&gone).await.1["verdict"]["verdict"],
            "anchor_missing"
        );
    }

    common::purge_record(&common::db().await, id).await;
}

/// A merge carries the duplicate's identifiers and addresses onto the
/// survivor, and the duplicate stops being served.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn merge_moves_the_duplicates_data_to_the_survivor() {
    let app = common::create_test_router().await;
    let main = create(
        &app,
        &json!({ "name": { "family": family("Mrgmain"), "given": ["A"] }, "gender": "female" }),
    )
    .await;
    let ssn = format!("SSN-{}", Uuid::new_v4());
    let duplicate = create(
        &app,
        &json!({
            "name": { "family": family("Mrgdup"), "given": ["B"] },
            "gender": "female",
            "identifiers": [{ "identifier_type": "SSN", "system": "http://hl7.org/fhir/sid/us-ssn", "value": ssn }],
            "addresses": [{ "line1": "3 Merge Lane", "city": "York", "postal_code": "YO1 1AA", "country": "GB" }]
        }),
    )
    .await;

    let (status, merged) = call(
        &app,
        "POST",
        "/api/workers/merge",
        Some(&json!({ "main_worker_id": main, "duplicate_worker_id": duplicate, "merge_reason": "test" })),
    )
    .await;
    assert!(status.is_success(), "{status} {merged}");

    let (status, survivor) = call(&app, "GET", &format!("/api/workers/{main}"), None).await;
    assert_eq!(status, StatusCode::OK);
    let text = survivor.to_string();
    assert!(text.contains(&ssn), "identifier moved: {text}");
    assert!(text.contains("3 Merge Lane"), "address moved: {text}");

    let (status, _) = call(&app, "GET", &format!("/api/workers/{duplicate}"), None).await;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "the duplicate is no longer served"
    );

    let db = common::db().await;
    common::purge_record(&db, main).await;
    common::purge_record(&db, duplicate).await;
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn audit_listings_answer() {
    let app = common::create_test_router().await;
    let (status, recent) = call(&app, "GET", "/api/audit/recent?limit=5", None).await;
    assert_eq!(status, StatusCode::OK, "{recent}");
    assert!(recent.is_array() || recent.is_object(), "{recent}");
    let (status, mine) = call(&app, "GET", "/api/audit/user?user_id=nobody&limit=5", None).await;
    assert_eq!(status, StatusCode::OK, "{mine}");
}

/// Every endpoint that names one worker answers an unknown id with `404`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn unknown_workers_are_404_everywhere() {
    let app = common::create_test_router().await;
    let id = Uuid::new_v4();
    let body = json!({ "name": { "family": "Nobody", "given": ["N"] }, "gender": "unknown" });
    for (method, uri, payload) in [
        ("GET", format!("/api/workers/{id}"), None),
        ("PUT", format!("/api/workers/{id}"), Some(&body)),
        ("DELETE", format!("/api/workers/{id}"), None),
        ("GET", format!("/api/workers/{id}/masked"), None),
        ("GET", format!("/api/workers/{id}/export"), None),
    ] {
        let (status, got) = call(&app, method, &uri, payload).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}: {got}");
    }
    // Erasure is the deliberate exception: idempotent, and it answers for
    // an unknown id, because the right to erasure outlives the record and a
    // 404 would tell a prober which ids exist.
    let (status, outcome) = call(&app, "POST", &format!("/api/workers/{id}/erase"), None).await;
    assert_eq!(status, StatusCode::OK, "{outcome}");
    assert_eq!(outcome["payload_erased"], false);
    assert_eq!(outcome["was_active"], false);
}
