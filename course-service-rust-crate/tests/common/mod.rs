//! Shared harness for the DB-backed integration suite.
//!
//! The tests are driven through `tower::ServiceExt::oneshot` against
//! the full Axum router with real PostgreSQL + Tantivy + the in-memory
//! event publisher. They require a running Postgres reachable via
//! `DATABASE_URL` (the rest of the env-var table from
//! `Config::from_env` applies). All integration tests are tagged with
//! `#[ignore]` so `cargo test --lib` stays fast; opt in via
//! `cargo test --test api_integration_test -- --ignored`.

#![allow(dead_code)]

use std::sync::OnceLock;

use axum::Router;
use chrono::Utc;
use serde_json::{Value, json};
use tempfile::TempDir;

use course_service::{
    api::rest::{AppState, create_router},
    config::Config,
    db::create_connection,
    matching::CourseMatcher,
    search::SearchEngine,
};

/// Held for the lifetime of the process so the search index dir
/// doesn't disappear before the tests finish.
fn index_dir() -> &'static TempDir {
    static DIR: OnceLock<TempDir> = OnceLock::new();
    DIR.get_or_init(|| TempDir::new().expect("create search index dir"))
}

/// Build an `AppState` against the env-configured Postgres + a
/// per-process temp Tantivy index. Panics with a clear message if the
/// DB isn't reachable.
pub async fn create_test_app_state() -> AppState {
    let mut config = Config::from_env().expect("load test config from env");
    config.search.index_path = index_dir().path().to_string_lossy().into_owned();

    let db = create_connection(&config.database)
        .await
        .expect(
            "Postgres connection failed — set DATABASE_URL to a running, migrated DB before \
             running integration tests",
        );

    let search_engine = SearchEngine::new(&config.search.index_path)
        .expect("create Tantivy search engine");

    let matcher = CourseMatcher::new(config.matching.clone());

    AppState::new(db, search_engine, matcher, config)
}

/// Convenience wrapper: build the test [`AppState`] and mount it on the
/// full production router, ready to drive via `oneshot`.
pub async fn create_test_router() -> Router {
    let state = create_test_app_state().await;
    create_router(state)
}

/// Generate a per-test unique course name so concurrent tests don't
/// step on each other inside the shared Postgres instance.
pub fn unique_name(suffix: &str) -> String {
    let ts = Utc::now().timestamp_micros();
    format!("Integration {suffix} {ts}")
}

/// Build a minimal valid Course JSON body. The id is the all-zeros
/// sentinel so the service generates a fresh UUID; `name` is unique.
pub fn course_json(suffix: &str) -> Value {
    json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": unique_name(suffix),
        "course_code": "TEST101",
        "status": "published",
    })
}
