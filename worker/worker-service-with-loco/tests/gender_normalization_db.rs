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

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, Statement};
use uuid::Uuid;

/// The migration's own SQL, so the test exercises what ships.
const NORMALIZE_SQL: &str =
    include_str!("../migrations/2026072300000002_normalize_worker_gender_case/up.sql");

/// The constraint PostgreSQL auto-names for the inline CHECK in
/// `migrations/2024122800000002_create_workers/up.sql`.
const CONSTRAINT: &str = "workers_gender_check";

/// The vocabulary the constraint admits.
const VOCABULARY: &str = "('male', 'female', 'other', 'unknown')";

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_raw(Statement::from_string(
        db.get_database_backend(),
        sql.to_string(),
    ))
    .await
    .unwrap_or_else(|e| panic!("exec failed: {sql}\n{e}"));
}

/// A legacy capitalized row is normalized, and the result is provably
/// constraint-legal.
#[tokio::test]
#[ignore = "requires DATABASE_URL to a migrated Postgres"]
async fn normalizes_legacy_capitalized_gender_rows() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let db = Database::connect(&url).await.expect("connect");
    let planted = Uuid::new_v4();

    // Reproduce the affected deployment: a `workers` table with no
    // gender constraint, holding a row the old writer produced.
    exec(
        &db,
        &format!("ALTER TABLE workers DROP CONSTRAINT IF EXISTS {CONSTRAINT}"),
    )
    .await;
    exec(
        &db,
        &format!("INSERT INTO workers (id, gender) VALUES ('{planted}', 'Male')"),
    )
    .await;

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
        &format!("ALTER TABLE workers ADD CONSTRAINT {CONSTRAINT} CHECK (gender IN {VOCABULARY})"),
    )
    .await;

    // Idempotent: a second run touches nothing (and stays legal).
    exec(&db, NORMALIZE_SQL).await;
    assert_eq!(gender_of(&db, planted).await, "male", "re-run is a no-op");

    exec(&db, &format!("DELETE FROM workers WHERE id = '{planted}'")).await;
}

/// Read one worker's stored gender string.
async fn gender_of(db: &DatabaseConnection, id: Uuid) -> String {
    let row = db
        .query_one_raw(Statement::from_string(
            db.get_database_backend(),
            format!("SELECT gender FROM workers WHERE id = '{id}'"),
        ))
        .await
        .expect("query")
        .expect("the planted row exists");
    row.try_get("", "gender").expect("gender column")
}
