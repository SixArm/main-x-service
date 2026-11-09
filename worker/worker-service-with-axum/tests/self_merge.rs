//! A merge of a record into itself is refused with `422` before anything
//! is read (SEC-B5). Without the refusal, `main == duplicate` applied the
//! survivor and then soft-deleted the same id, so the record was gone.
//!
//! No database is needed: the real repository is given a lazy pool aimed
//! at a port nothing listens on, so any read would fail with `500`, and a
//! `422` proves the refusal came first.

use std::time::Duration;

use axa::extract::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use sqlx::postgres::PgPoolOptions;
use worker_service::api::rest::AppState;
use worker_service::config::Config;
use worker_service::matching::ProbabilisticMatcher;
use worker_service::search::SearchEngine;

fn unreachable_state(index: &tempfile::TempDir) -> AppState {
    let pool = PgPoolOptions::new()
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/none")
        .expect("a lazy pool never connects up front");
    let config = Config::default();
    let matcher = ProbabilisticMatcher::new(config.matching.clone());
    let search = SearchEngine::new(index.path()).expect("temp search index");
    AppState::new(pool, search, matcher, config)
}

fn request(main: uuid::Uuid, duplicate: uuid::Uuid) -> worker_service::models::MergeRequest {
    serde_json::from_value(serde_json::json!({
        "main_worker_id": main,
        "duplicate_worker_id": duplicate,
    }))
    .expect("a merge request")
}

#[tokio::test]
async fn merging_a_record_into_itself_is_refused_before_any_read() {
    let index = tempfile::tempdir().unwrap();
    let id = uuid::Uuid::new_v4();
    let response = worker_service::api::rest::handlers::merge_workers(
        State(unreachable_state(&index)),
        worker_service::api::rest::auth::MaybeAuthUser(None),
        axa::client::ClientAddr(None),
        Json(request(id, id)),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn two_different_records_reach_the_database() {
    // The control: different ids get past the guard to the first read,
    // which fails here because nothing listens.
    let index = tempfile::tempdir().unwrap();
    let response = worker_service::api::rest::handlers::merge_workers(
        State(unreachable_state(&index)),
        worker_service::api::rest::auth::MaybeAuthUser(None),
        axa::client::ClientAddr(None),
        Json(request(uuid::Uuid::new_v4(), uuid::Uuid::new_v4())),
    )
    .await
    .into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
