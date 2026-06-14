//! Shared helpers for the REST integration tests.
//!
//! Builds a real [`AppState`] / [`Router`](axum::Router) from the
//! environment config (database + search index) and provides a
//! collision-free name generator so concurrently-run tests do not
//! clash on unique fields.

use axum::Router;
use chrono::Utc;
use person_service::{
    api::rest::{AppState, create_router},
    config::Config,
    db::create_connection,
    matching::ProbabilisticMatcher,
    search::SearchEngine,
};

/// Create a test application state for integration tests
pub async fn create_test_app_state() -> AppState {
    let config = Config::from_env().expect("Failed to load test config");

    let db = create_connection(&config.database)
        .await
        .expect("Failed to create database connection");

    let search_engine =
        SearchEngine::new(&config.search.index_path).expect("Failed to create search engine");

    let matcher = ProbabilisticMatcher::new(config.matching.clone());

    AppState::new(db, search_engine, matcher, config)
}

/// Create a test router with test application state
pub async fn create_test_router() -> Router {
    let state = create_test_app_state().await;
    create_router(state)
}

/// Create a unique test person name to avoid conflicts
pub fn unique_person_name(suffix: &str) -> String {
    let timestamp = Utc::now().timestamp_micros();
    format!("TestPerson{}_{}", suffix, timestamp)
}
