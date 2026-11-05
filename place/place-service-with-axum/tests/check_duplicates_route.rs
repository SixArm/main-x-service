//! Service route test for `POST /api/places/check-duplicates` (entity
//! spec E-1).
//!
//! Three docs once disagreed on this endpoint's path
//! (`agents/restful.md` said `/api/places/duplicates`; the service spec
//! and the front-end's deferred task both said `check-duplicates`); the
//! code was confirmed to serve `check-duplicates` and the losing doc was
//! fixed (2026-06-13), and the front-end client's matching bug was fixed
//! the same day — but no **service** route test ever pinned the path
//! itself, so a future regression on either surface (`src/api/rest/mod.rs`'s
//! two route tables, or the handler's own `#[utoipa::path]` annotation)
//! could silently drift again. This closes that gap.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test check_duplicates_route -- --ignored`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use place_service::api::rest::{AppState, create_router};
use place_service::config::Config;
use place_service::db::create_connection;
use place_service::matching::PlaceMatcher;
use place_service::models::place::Place;
use place_service::search::SearchEngine;
use serde_json::Value;
use tower::ServiceExt;

/// Build the real router against the environment-configured database and
/// a temp search index — same construction `api_integration_test.rs` uses.
async fn test_router() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    let search_engine = SearchEngine::new(&config.search.index_path).expect("search engine");
    let matcher = PlaceMatcher::new(&config.matching);
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection — set DATABASE_URL to a running, migrated database");
    create_router(AppState::new(db, search_engine, matcher, config))
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test check_duplicates_route -- --ignored"]
async fn check_duplicates_is_served_at_the_documented_path() {
    let app = test_router().await;

    let place = Place::new("Central Park Route Test");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/places/check-duplicates")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&place).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Not 404: the route exists at this exact path (pins E-1's fix — the
    // handler used to be reachable only at the doc-drifted
    // `/api/places/duplicates`, which would 404 here).
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["success"], true, "{body}");
    assert!(
        body["data"]["duplicates_found"].is_boolean(),
        "response should carry the DuplicateCheckResponse envelope, got {body}"
    );
    assert!(body["data"]["candidates"].is_array(), "{body}");

    // The doc-drifted path from before the fix must NOT serve
    // check-duplicates — pinning that the old name is gone, not merely
    // that the new one works. It's a `405`, not a `404`: Axum's router
    // matches `/api/places/duplicates` onto the `/api/places/{id}`
    // pattern (treating "duplicates" as an id), which has GET/PUT/DELETE
    // handlers but none for POST.
    let stale = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/places/duplicates")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&place).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        stale.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "the pre-fix doc-drifted path must not serve check-duplicates"
    );
}
