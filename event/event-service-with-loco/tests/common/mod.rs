//! Shared helpers for integration tests.
//!
//! These tests need a real `PostgreSQL` — point `DATABASE_URL` at one
//! before running `cargo test --test api_integration_test`. The
//! helpers are async because [`event_service::db::create_connection`]
//! is.

use axum::Router;
use event_service::{
    api::rest::{AppState, create_router},
    config::Config,
    db,
    matching::ProbabilisticMatcher,
    search::SearchEngine,
};

/// Boot a test [`AppState`] against the configured database.
pub async fn create_test_app_state() -> AppState {
    let config = Config::from_env().expect("Failed to load test config");
    let db = db::create_connection(&config.database)
        .await
        .expect("Failed to connect to database");
    // Each test gets its own in-memory search index dir.
    let tmp = tempfile::tempdir().expect("tempdir for search index");
    let search_engine = SearchEngine::new(tmp.path()).expect("Failed to create search engine");
    let matcher = ProbabilisticMatcher::new(config.matching.clone());
    // Keep the tempdir alive for the duration of the test by leaking it.
    std::mem::forget(tmp);
    AppState::new(db, search_engine, matcher, config)
}

/// Build a fully-wired test [`Router`] over a fresh test [`AppState`].
pub async fn create_test_router() -> Router {
    let state = create_test_app_state().await;
    create_router(state)
}

/// Unique title suffix to avoid clashes across parallel tests.
pub fn unique_event_name(suffix: &str) -> String {
    let ts = chrono::Utc::now().timestamp_micros();
    format!("TestEvent_{suffix}_{ts}")
}
