//! Shared harness for the DB-backed integration suite.
//!
//! The tests are driven through `tower::ServiceExt::oneshot` against
//! the full Axum router with real `PostgreSQL` + Tantivy + the in-memory
//! event publisher. They require a running Postgres reachable via
//! `DATABASE_URL` (the rest of the env-var table from
//! `Config::from_env` applies). All integration tests are tagged with
//! `#[ignore]` so `cargo test --lib` stays fast; opt in via
//! `cargo test --test api_integration_test -- --ignored`.

#![allow(dead_code)]

use std::sync::OnceLock;

use axum::Router;
use serde_json::{Value, json};
use tempfile::TempDir;

use axa::testing::TestApp;
use course_service::{api::rest::AppState, app::App};

/// Held for the lifetime of the process so the search index dir
/// doesn't disappear before the tests finish.
fn index_dir() -> &'static TempDir {
    static DIR: OnceLock<TempDir> = OnceLock::new();
    DIR.get_or_init(|| TempDir::new().expect("create search index dir"))
}

/// Boot the real application: the router production serves, plus the
/// [`AppState`] that router holds, so a test seeding through the state
/// writes to the same search index the routes read.
pub async fn boot() -> (Router, AppState) {
    // SAFETY: the DB-gated suites run with `--test-threads=1`
    // (`ci-check.sh test-db`), so no other thread reads the environment
    // concurrently with this write.
    unsafe {
        std::env::set_var("SEARCH_INDEX_PATH", index_dir().path());
    }
    let boot = App::boot_test()
        .await
        .expect("boot the app: set DATABASE_URL to a running PostgreSQL");
    let state = boot
        .app_context
        .shared_store
        .get::<AppState>()
        .expect("after_routes stores the AppState");
    (boot.router.expect("boot_test builds the router"), state)
}

/// The full production router, ready to drive via `oneshot`.
pub async fn create_test_router() -> Router {
    boot().await.0
}

/// Generate a per-test unique course name so tests don't step on each
/// other inside the shared Postgres instance.
///
/// The random token comes **first** and there is no shared prefix. Both
/// details are load-bearing, and both were learned the hard way:
///
/// - The name used to be `Integration <suffix> <micros>`. Consecutive
///   microsecond timestamps share nearly every leading digit, so two such
///   names scored ~0.92 on Jaro-Winkler — over the match threshold. Every
///   create after the first came back `409 duplicate_candidate` against a
///   record an earlier test had left behind.
/// - Swapping the timestamp for a random UUID was not enough: with the
///   constant `Integration ` prefix still in front, Jaro-Winkler's prefix
///   bonus kept the score at ~0.88 — still a match.
///
/// The duplicate detector is not wrong here; the fixtures were. A name
/// that differs from the first character has nothing for the prefix bonus
/// to work with.
pub fn unique_name(suffix: &str) -> String {
    let token = uuid::Uuid::new_v4().simple().to_string();
    format!("{token} {suffix}")
}

/// Build a minimal valid Course JSON body with a unique `name`.
///
/// `id` is **omitted**, which is how the API mints one: `Course::id`
/// carries `#[serde(default = "Uuid::new_v4")]`, and a serde default only
/// applies to an *absent* field. This body used to send the all-zeros
/// UUID as a "generate one for me" sentinel, which the service dutifully
/// stored — so the first test claimed the nil id and every later create
/// died on `duplicate key value violates unique constraint
/// "courses_pkey"`. (The handler now also mints on an explicit nil, so
/// the sentinel would work too; omitting it is still the contract.)
pub fn course_json(suffix: &str) -> Value {
    json!({
        "name": unique_name(suffix),
        "course_code": "TEST101",
        "status": "published",
    })
}

/// Seed a minimal course directly through the repository + search index,
/// bypassing `POST /api/courses`' real-time duplicate check entirely.
///
/// Used to build deliberately near-duplicate fixtures for the review-queue
/// tests: two records sharing a similar `name` would otherwise `409` on
/// the second create, exactly the fixture-collision class documented in
/// the family's `reference_realtime_dedup_breaks_test_fixtures` memory —
/// the fix there, as here, is to write through the repository (and, since
/// indexing normally happens in the create handler rather than the
/// repository, the search engine too) instead of through the guarded HTTP
/// endpoint.
pub async fn seed_course(state: &course_service::api::rest::AppState, name: &str) -> uuid::Uuid {
    use course_service::models::Course;
    let course = Course::new(name);
    let created = state
        .course_repository
        .create(&course)
        .await
        .expect("seed course directly via the repository");
    state
        .search_engine
        .index_course(&created)
        .expect("index seeded course");
    created.id
}

/// Seed `course` as given (identifiers and all) directly through the
/// repository + search index, bypassing the create endpoint's duplicate
/// check, as [`seed_course`] does for a bare name.
pub async fn seed_full_course(
    state: &course_service::api::rest::AppState,
    course: &course_service::models::Course,
) -> uuid::Uuid {
    let created = state
        .course_repository
        .create(course)
        .await
        .expect("seed course directly via the repository");
    state
        .search_engine
        .index_course(&created)
        .expect("index seeded course");
    created.id
}
