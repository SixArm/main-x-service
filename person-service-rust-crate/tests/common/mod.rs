//! Common test utilities for integration tests

use person_service::{
    config::Config,
    db::create_connection,
    search::SearchEngine,
    matching::ProbabilisticMatcher,
    api::rest::{AppState, create_router},
};
use axum::Router;

/// Create a test application state for integration tests
pub async fn create_test_app_state() -> AppState {
    let config = Config::from_env().expect("Failed to load test config");

    let db = create_connection(&config.database)
        .await
        .expect("Failed to create database connection");

    let search_engine = SearchEngine::new(&config.search.index_path)
        .expect("Failed to create search engine");

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
    use chrono::Utc;
    let timestamp = Utc::now().timestamp_micros();
    format!("TestPerson{}_{}", suffix, timestamp)
}
