//! DB-gated pin for the `workers.gender` case-normalization data
//! migration (`m20260723_000002_normalize_worker_gender_case`).
//!
//! Run with a migrated Postgres:
//!
//! ```text
//! DATABASE_URL=postgres://…/worker_service_test cargo test --test gender_normalization_db -- --ignored
//! ```
//!
//! The legacy rows this migration repairs **cannot be created on a
//! constrained schema** — the CHECK constraint is exactly what rejected
//! them. So the test reproduces the affected deployment shape: it drops
//! the constraint, plants a capitalized row, runs the migration's real
//! SQL (via `include_str!`, so the test and the migration can never
//! drift), and then asserts the repair by **re-adding the constraint** —
//! which only succeeds if every row is now legal. The constraint is
//! restored and the planted row removed either way.

#![allow(clippy::expect_used)]

use sqlx::PgPool;
use uuid::Uuid;

/// The migration's own SQL, so the test exercises what ships.
const NORMALIZE_SQL: &str =
    include_str!("../migrations/2026072300000002_normalize_worker_gender_case.up.sql");

// The constraint is `workers_gender_check`, the name PostgreSQL gives the
// inline CHECK in `migrations/2024122800000002_create_workers.up.sql`, and
// its vocabulary is `('male', 'female', 'other', 'unknown')`. Both are
// written into the statements below, which sqlx takes only as literals.

/// Run one statement, naming it if it fails.
async fn exec(db: &PgPool, sql: &'static str) {
    sqlx::query(sql)
        .execute(db)
        .await
        .unwrap_or_else(|e| panic!("exec failed: {sql}\n{e}"));
}

/// A legacy capitalized row is normalized, and the result is provably
/// constraint-legal.
#[tokio::test]
#[ignore = "requires DATABASE_URL to a migrated Postgres"]
async fn normalizes_legacy_capitalized_gender_rows() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let db = PgPool::connect(&url).await.expect("connect");
    let planted = Uuid::new_v4();

    // Reproduce the affected deployment: a `workers` table with no
    // gender constraint, holding a row the old writer produced.
    exec(
        &db,
        "ALTER TABLE workers DROP CONSTRAINT IF EXISTS workers_gender_check",
    )
    .await;
    sqlx::query("INSERT INTO workers (id, gender) VALUES ($1, 'Male')")
        .bind(planted)
        .execute(&db)
        .await
        .expect("plant the legacy row");

    // Sanity: the planted row is exactly the shape the constraint rejects.
    let before = gender_of(&db, planted).await;
    assert_eq!(before, "Male", "the legacy row was planted as written");

    // Run the migration's real SQL.
    exec(&db, NORMALIZE_SQL).await;

    let after = gender_of(&db, planted).await;
    assert_eq!(after, "male", "the legacy value is normalized in place");

    // The proof: re-adding the constraint succeeds only if every row is
    // legal now. This would fail on the pre-migration data.
    exec(
        &db,
        "ALTER TABLE workers ADD CONSTRAINT workers_gender_check \
         CHECK (gender IN ('male', 'female', 'other', 'unknown'))",
    )
    .await;

    // Idempotent: a second run touches nothing (and stays legal).
    exec(&db, NORMALIZE_SQL).await;
    assert_eq!(gender_of(&db, planted).await, "male", "re-run is a no-op");

    sqlx::query("DELETE FROM workers WHERE id = $1")
        .bind(planted)
        .execute(&db)
        .await
        .expect("remove the planted row");
}

/// Read one worker's stored gender string.
async fn gender_of(db: &PgPool, id: Uuid) -> String {
    sqlx::query_scalar("SELECT gender FROM workers WHERE id = $1")
        .bind(id)
        .fetch_optional(db)
        .await
        .expect("query")
        .expect("the planted row exists")
}
