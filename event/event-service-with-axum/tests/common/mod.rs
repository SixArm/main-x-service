//! Shared helpers for integration tests.
//!
//! These tests need a real `PostgreSQL` — point `DB_URL` at one
//! before running `cargo test --test api_integration_test`. The
//! helpers are async because [`event_service::db::create_connection`]
//! is.
//!
//! Each test binary compiles this module separately, so a helper used
//! by one suite is dead code in another (PRO-H11's `grpc_integration_test`
//! needs `create_test_app_state` but not `create_test_router`, which
//! only `api_integration_test` builds a REST router with). That is a
//! property of the layout, not a defect worth deleting helpers over —
//! same convention person-service's and worker-service's `tests/common/mod.rs`
//! already use.
#![allow(dead_code)]

use axa_test::TestApp;
use axum::Router;
use event_service::{
    api::rest::AppState, app::App, config::Config, db, matching::ProbabilisticMatcher,
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

/// Boot the real application, the router production serves, over a
/// private search index.
pub async fn create_test_router() -> Router {
    let index = tempfile::tempdir().expect("tempdir for search index");
    // SAFETY: the DB-gated suites run with `--test-threads=1`
    // (`ci-check.sh test-db`), so no other thread reads the environment
    // concurrently with this write.
    unsafe {
        std::env::set_var("SEARCH_INDEX_PATH", index.path());
    }
    // Keep the tempdir alive for the duration of the test by leaking it.
    std::mem::forget(index);
    App::boot_test()
        .await
        .expect("boot the app: set DB_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router")
}

/// Unique title suffix to avoid clashes across parallel tests.
pub fn unique_event_name(suffix: &str) -> String {
    let ts = chrono::Utc::now().timestamp_micros();
    format!("TestEvent_{suffix}_{ts}")
}
