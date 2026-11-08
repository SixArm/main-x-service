//! Shared setup helpers for the integration test suite.
//!
//! Builds a real [`AppState`] and Axum [`Router`] from the environment-driven
//! [`Config`] (so tests run against the same wiring as production) and provides
//! a name-uniqueness helper so parallel tests do not collide on worker data.

//! Each test binary compiles this module separately, so a helper used by
//! one suite is dead code in another. That is a property of the layout,
//! not a defect worth deleting helpers over.
#![allow(dead_code)]

use axa::{AppContext, Environment, config::Defaults, testing::TestApp};
use axum::Router;
use worker_service::{
    api::rest::AppState, app::App, config::Config, db::create_connection,
    matching::ProbabilisticMatcher, search::SearchEngine,
};

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
    // Tantivy opens an existing directory rather than creating one, and no
    // fixture ever made this path — so every integration test in this crate
    // panicked with `DoesNotExist` before reaching its assertions, and the
    // whole target could not run against a database. Creating it here keeps
    // the path environment-driven (CI can point it anywhere) while removing
    // the unstated precondition.
    std::fs::create_dir_all(&config.search.index_path)
        .expect("Failed to create the search index directory");
    let search_engine =
        SearchEngine::new(&config.search.index_path).expect("Failed to create search engine");

    // Create matcher
    let matcher = ProbabilisticMatcher::new(config.matching.clone());

    // Create application state
    AppState::new(db, search_engine, matcher, config)
}

/// Boot the real application: the router production serves.
pub async fn create_test_router() -> Router {
    let config = Config::from_env().expect("Failed to load test config");
    // `after_routes` opens the index but does not create its directory.
    std::fs::create_dir_all(&config.search.index_path)
        .expect("Failed to create the search index directory");
    App::boot_test()
        .await
        .expect("boot the app: set DATABASE_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router")
}

/// The real application router over a **lazy** pool that never connects
/// and a throwaway search index, so no live database is required.
///
/// Suitable only for tests that assert on routing/extractor behaviour *before*
/// any database access (e.g. a malformed path that the `Path<Uuid>` extractor
/// rejects with `400`). Any handler that actually queries the database will
/// fail against this router.
pub async fn create_test_router_no_db() -> Router {
    let index = tempfile::tempdir().expect("Failed to create temp dir for search index");
    // SAFETY: the only un-gated test using this helper runs before any other
    // thread in this binary reads the variable, and the DB-gated suites run
    // with `--test-threads=1` (`ci-check.sh test-db`).
    unsafe {
        std::env::set_var("SEARCH_INDEX_PATH", index.path());
    }
    // Keep the temp index alive for the lifetime of the test by leaking it;
    // tests are short-lived processes and this avoids a dangling index dir.
    std::mem::forget(index);
    let environment = Environment::Test;
    let config = axa::config::Config::from_env(&environment, &Defaults::default())
        .expect("Failed to load test config");
    // Nothing connects until a query runs, and nothing here runs one.
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://disconnected.invalid/none")
        .expect("a lazy pool");
    App::router(&AppContext::new(environment, db, config))
        .await
        .expect("Failed to build the router")
}

/// Returns a worker family name unique to this instant
/// (`TestWorker{suffix}_{micros}`) so tests sharing one database do not match
/// each other's records during search/dedup assertions.
pub fn unique_worker_name(suffix: &str) -> String {
    let timestamp = chrono::Utc::now().timestamp_micros();
    format!("TestWorker{suffix}_{timestamp}")
}

/// A direct database connection, for assertions that must not go through
/// the API.
///
/// GDPR Art. 17 erasure soft-deletes the worker, so every worker-scoped
/// endpoint afterwards returns `404` — including the assessments list. A
/// `404` proves only that the route is unreachable, not that the rows are
/// gone, and "the data is unreachable" is exactly the weaker claim
/// erasure is not allowed to settle for. Ground truth needs SQL.
pub async fn db() -> sqlx::PgPool {
    let config = Config::from_env().expect("Failed to load test config");
    create_connection(&config.database)
        .await
        .expect("Failed to create database connection")
}

/// Count rows from a `SELECT count(*) AS n … WHERE <col> = $1` query.
///
/// Lives here rather than nested inside a test so it is defined before
/// use (clippy's `items_after_statements`) and is shared by every
/// ground-truth assertion.
pub async fn count_rows(conn: &sqlx::PgPool, sql: &'static str, id: uuid::Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(conn)
        .await
        .expect("count query")
}

/// Hard-delete a record and everything hanging off it.
///
/// Used by the tamper tests, which deliberately corrupt a record to prove
/// detection works and must not leave that corruption behind. The database
/// is shared by every DB-gated target in the crate, so a tampered row left
/// in place surfaces later as a failure in an unrelated test — which is
/// exactly what happened before this existed, and reads as a product
/// defect rather than test residue.
///
/// A hard delete, not the API's soft delete: the point is to remove the
/// evidence, and a soft-deleted row still carries its mismatched hash.
pub async fn purge_record(conn: &sqlx::PgPool, id: uuid::Uuid) {
    for sql in [
        "DELETE FROM worker_assessments WHERE worker_id = $1",
        // Deleting the parent cascades to the child tables that declare
        // a foreign key; any table that does not is listed above.
        "DELETE FROM workers WHERE id = $1",
    ] {
        sqlx::query(sql)
            .bind(id)
            .execute(conn)
            .await
            .expect("purge test record");
    }
}
