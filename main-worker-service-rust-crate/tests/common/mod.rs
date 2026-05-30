//! Common test utilities for integration tests

use main_worker_service::{
    config::Config,
    db::create_pool,
    search::SearchEngine,
    matching::ProbabilisticMatcher,
    api::rest::{AppState, create_router},
};
use axum::Router;

/// Create a test application state for integration tests
pub fn create_test_app_state() -> AppState {
    // Load test configuration
    let config = Config::from_env().expect("Failed to load test config");

    // Create database pool
    let db_pool = create_pool(&config.database)
        .expect("Failed to create database pool");

    // Create search engine
    let search_engine = SearchEngine::new(&config.search.index_path)
        .expect("Failed to create search engine");

    // Create matcher
    let matcher = ProbabilisticMatcher::new(config.matching.clone());

    // Create application state
    AppState::new(db_pool, search_engine, matcher, config)
}

/// Create a test router with test application state
pub fn create_test_router() -> Router {
    let state = create_test_app_state();
    create_router(state)
}

/// Create a unique test worker name to avoid conflicts
pub fn unique_worker_name(suffix: &str) -> String {
    use chrono::Utc;
    let timestamp = Utc::now().timestamp_micros();
    format!("TestWorker{}_{}", suffix, timestamp)
}
