//! Shared setup helpers for the integration test suite.
//!
//! Builds a real [`AppState`] and Axum [`Router`] from the environment-driven
//! [`Config`] (so tests run against the same wiring as production) and provides
//! a name-uniqueness helper so parallel tests do not collide on worker data.

use worker_service::{
    config::Config,
    db::create_connection,
    search::SearchEngine,
    matching::ProbabilisticMatcher,
    api::rest::{AppState, create_router},
};
use axum::Router;

/// Builds a fully wired [`AppState`] for tests: loads config from the
/// environment, connects to the (test) database, opens the search index, and
/// constructs the probabilistic matcher. Panics on any setup failure so a
/// misconfigured test environment fails loudly.
pub async fn create_test_app_state() -> AppState {
    // Load test configuration
    let config = Config::from_env().expect("Failed to load test config");

    // Create database connection
    let db = create_connection(&config.database)
        .await
        .expect("Failed to create database connection");

    // Create search engine
    let search_engine = SearchEngine::new(&config.search.index_path)
        .expect("Failed to create search engine");

    // Create matcher
    let matcher = ProbabilisticMatcher::new(config.matching.clone());

    // Create application state
    AppState::new(db, search_engine, matcher, config)
}

/// Builds the full application [`Router`] over a fresh test [`AppState`],
/// ready to drive with `tower`'s `oneshot`.
pub async fn create_test_router() -> Router {
    let state = create_test_app_state().await;
    create_router(state)
}

/// Returns a worker family name unique to this instant
/// (`TestWorker{suffix}_{micros}`) so tests sharing one database do not match
/// each other's records during search/dedup assertions.
pub fn unique_worker_name(suffix: &str) -> String {
    use chrono::Utc;
    let timestamp = Utc::now().timestamp_micros();
    format!("TestWorker{}_{}", suffix, timestamp)
}
