//! Shared helpers for the REST integration tests.
//!
//! Builds a real [`AppState`] / [`Router`](axum::Router) from the
//! environment config (database + search index) and provides a
//! collision-free name generator so concurrently-run tests do not
//! clash on unique fields.
//!
//! Each test binary compiles this module separately, so a helper used by
//! one suite is dead code in another (the enforcement binary needs the
//! router and the name generator, not the row-counting helpers). That is
//! a property of the layout, not a defect worth deleting helpers over.
#![allow(dead_code)]

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
    format!("TestPerson{suffix}_{timestamp}")
}

/// A direct database connection, for assertions that must not go through
/// the API.
///
/// GDPR Art. 17 erasure soft-deletes the person, so a subsequent `GET`
/// may return `404` — and an assertion guarded by "if the read succeeded"
/// then passes without checking anything. A `404` proves only that the
/// record is unreachable, not that the data is gone, and "unreachable" is
/// exactly the weaker claim erasure is not allowed to settle for. Ground
/// truth needs SQL.
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
pub async fn purge_record(conn: &sea_orm::DatabaseConnection, id: uuid::Uuid) {
    use sea_orm::ConnectionTrait as _;
    for sql in [
        // Deleting the parent cascades to the child tables that declare
        // a foreign key; any table that does not is listed above.
        "DELETE FROM persons WHERE id = $1",
    ] {
        conn.execute(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            sql,
            [id.into()],
        ))
        .await
        .expect("purge test record");
    }
}
