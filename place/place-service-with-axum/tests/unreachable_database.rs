//! A database failure on the list and batch-deduplicate paths answers
//! `500`, not an empty `200` that would tell the caller there are no
//! places; and a `max_candidates` above the cap is refused with `422`
//! before anything is read.
//!
//! No database is needed: the real repository is given a lazy pool aimed
//! at a port nothing listens on, so every query fails.

use std::time::Duration;

use axa::extract::{Json, Query};
use axum::extract::{OriginalUri, State};
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use place_service::api::rest::handlers::{
    BatchDeduplicationRequest, DEDUPLICATE_MAX_CANDIDATES, ListQuery, deduplicate, list_places,
};
use place_service::api::rest::state::AppState;
use place_service::config::Config;
use place_service::matching::PlaceMatcher;
use place_service::search::SearchEngine;
use sqlx::postgres::PgPoolOptions;

fn unreachable_state(index: &tempfile::TempDir) -> AppState {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("a lazy pool never connects up front");
    let config = Config::default();
    let matcher = PlaceMatcher::new(&config.matching);
    let search = SearchEngine::new(index.path()).expect("temp search index");
    AppState::new(pool, search, matcher, config)
}

#[tokio::test]
async fn a_list_that_cannot_reach_the_database_is_a_500_not_an_empty_page() {
    let index = tempfile::tempdir().unwrap();
    let uri: Uri = "/api/places".parse().unwrap();
    let response = list_places(
        State(unreachable_state(&index)),
        OriginalUri(uri),
        Query(ListQuery {
            limit: None,
            cursor: None,
            mask_sensitive: None,
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn a_deduplicate_that_cannot_reach_the_database_is_a_500() {
    let index = tempfile::tempdir().unwrap();
    let response = deduplicate(
        State(unreachable_state(&index)),
        Json(BatchDeduplicationRequest {
            threshold: None,
            max_candidates: Some(10),
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn max_candidates_above_the_cap_is_refused_before_any_read() {
    let index = tempfile::tempdir().unwrap();
    // The database is unreachable, so a 422 here proves the refusal came
    // first: a read would have answered 500.
    let response = deduplicate(
        State(unreachable_state(&index)),
        Json(BatchDeduplicationRequest {
            threshold: None,
            max_candidates: Some(DEDUPLICATE_MAX_CANDIDATES + 1),
        }),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

fn merge_request(
    main: uuid::Uuid,
    duplicate: uuid::Uuid,
) -> place_service::models::merge::MergeRequest {
    serde_json::from_value(serde_json::json!({
        "main_place_id": main,
        "duplicate_place_id": duplicate,
    }))
    .expect("a merge request")
}

/// A merge of a record into itself is refused with `422` before any read
/// (SEC-B5): with `main == duplicate` the merge applied the survivor and
/// then soft-deleted the same id. The database is unreachable, so a read
/// would have answered `500`.
#[tokio::test]
async fn merging_a_record_into_itself_is_refused_before_any_read() {
    let index = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4();
    let response = place_service::api::rest::handlers::merge_places(
        State(unreachable_state(&index)),
        Json(merge_request(id, id)),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
