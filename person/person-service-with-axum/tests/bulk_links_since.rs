//! `GET /api/persons/links?since=` — a caller-supplied instant. RFC3339
//! admits years past the nanosecond range that ends in 2262; such a value
//! used to panic the conversion (security invariant 2). A far-future
//! `since` is now an ordinary, empty pull, and a malformed one a `422`.
//! DB-gated.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

async fn status(app: &axum::Router, uri: &str) -> StatusCode {
    app.clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn far_future_since_is_an_empty_pull_not_a_panic() {
    let app = common::create_test_router().await;
    assert_eq!(
        status(&app, "/api/persons/links?since=2999-01-01T00:00:00Z").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, "/api/persons/links?since=9999-12-31T23:59:59Z").await,
        StatusCode::OK
    );
    assert_eq!(
        status(&app, "/api/persons/links?since=yesterday").await,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}
