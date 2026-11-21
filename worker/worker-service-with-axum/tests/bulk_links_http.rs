//! `GET /api/workers/links` — the reconciliation pull the link-graph
//! aggregator reads every `same_identity` edge through — with enforcement
//! off: it returns the edges, `since` filters them, a malformed `since` is
//! `422`, and every pull is audited. The privileged-read gate is in
//! `bulk_links_enforced.rs`. DB-gated.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use tower::ServiceExt;

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

#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn bulk_pull_returns_filters_refuses_bad_since_and_audits() {
    let app = common::create_test_router().await;
    let db = common::db().await;
    let (status, created) = call(
        &app,
        "POST",
        "/api/workers",
        Some(&json!({ "name": { "family": "Bulklinkqq", "given": ["A"] }, "gender": "unknown" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let worker: uuid::Uuid = created["id"].as_str().unwrap().parse().unwrap();
    let to_ref = format!("person:{}", uuid::Uuid::new_v4());
    let (status, link) = call(
        &app,
        "POST",
        &format!("/api/workers/{worker}/links"),
        Some(&json!({ "kind": "same_identity", "to_ref": to_ref })),
    )
    .await;
    assert!(status.is_success(), "{status} {link}");

    let audited_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE entity_type = 'worker_links_bulk'",
    )
    .fetch_one(&db)
    .await
    .unwrap();

    let (status, all) = call(&app, "GET", "/api/workers/links", None).await;
    assert_eq!(status, StatusCode::OK, "{all}");
    assert!(all["edges"].to_string().contains(&to_ref), "{all}");

    let (status, later) = call(
        &app,
        "GET",
        "/api/workers/links?since=2999-01-01T00:00:00Z",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !later["edges"].to_string().contains(&to_ref),
        "since filters: {later}"
    );

    let (status, _) = call(&app, "GET", "/api/workers/links?since=yesterday", None).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let audited_after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE entity_type = 'worker_links_bulk'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(
        audited_after - audited_before,
        2,
        "each successful pull is audited"
    );

    common::purge_record(&db, worker).await;
}
