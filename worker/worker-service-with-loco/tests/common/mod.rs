//! Shared setup helpers for the integration test suite.
//!
//! Builds a real [`AppState`] and Axum [`Router`] from the environment-driven
//! [`Config`] (so tests run against the same wiring as production) and provides
//! a name-uniqueness helper so parallel tests do not collide on worker data.

use axum::Router;
use worker_service::{
    api::rest::{AppState, create_router},
    config::Config,
    db::create_connection,
    matching::ProbabilisticMatcher,
    search::SearchEngine,
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

/// Builds the full application [`Router`] over a fresh test [`AppState`],
/// ready to drive with `tower`'s `oneshot`.
pub async fn create_test_router() -> Router {
    let state = create_test_app_state().await;
    create_router(state)
}

/// Builds the full application [`Router`] over an [`AppState`] backed by a
/// **disconnected** `SeaORM` connection and a throwaway search index — no live
/// database required.
///
/// Suitable only for tests that assert on routing/extractor behaviour *before*
/// any database access (e.g. a malformed path that the `Path<Uuid>` extractor
/// rejects with `400`). Any handler that actually queries the database will
/// fail against this router.
pub fn create_test_router_no_db() -> Router {
    let config = Config::default();
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir for search index");
    let search_engine = SearchEngine::new(temp_dir.path().to_str().unwrap())
        .expect("Failed to create search engine");
    let matcher = ProbabilisticMatcher::new(config.matching.clone());
    let db = sea_orm::DatabaseConnection::Disconnected;
    let state = AppState::new(db, search_engine, matcher, config);
    // Keep the temp index alive for the lifetime of the test by leaking it;
    // tests are short-lived processes and this avoids a dangling index dir.
    std::mem::forget(temp_dir);
    create_router(state)
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
pub async fn db() -> sea_orm::DatabaseConnection {
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
pub async fn count_rows(conn: &sea_orm::DatabaseConnection, sql: &str, id: uuid::Uuid) -> i64 {
    use sea_orm::ConnectionTrait as _;
    conn.query_one(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        sql,
        [id.into()],
    ))
    .await
    .expect("count query")
    .expect("one row")
    .try_get::<i64>("", "n")
    .expect("n")
}
