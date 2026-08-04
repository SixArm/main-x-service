//! DB-gated proof for the `seed_examples` task (EX-4,
//! `src/tasks/seed_examples.rs`): a first run seeds every fixture row
//! (including both halves of a documented duplicate pair — the whole
//! point of bypassing the create endpoint's duplicate detection), and a
//! second run is a no-op rather than doubling the table.
//!
//! Owns the whole `persons` table for its assertions (row counts), so it
//! truncates at the start — the same "clean slate" precedent
//! `tests/review_queue_db.rs` uses for `review_queue`.
//!
//! `#[ignore]`d: touches Postgres. Run with:
//! `cargo test --test seed_examples_db -- --ignored`
//! (or `scripts/ci-check.sh test-db person/person-service-with-loco`).

mod common;

use std::path::Path;

use person_service::db::models::persons;
use person_service::tasks::seed_examples::{FIXTURE_PATH, parse_fixture, seed};
use sea_orm::{ConnectionTrait, EntityTrait, PaginatorTrait};

#[tokio::test]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test --test seed_examples_db -- --ignored"]
async fn seed_examples_seeds_the_fixture_including_a_duplicate_pair_and_is_idempotent() {
    let db = common::db().await;

    // Clean slate: this test asserts on the whole table's row count, so
    // it needs sole ownership of the starting state regardless of what
    // ran before it in the shared test database.
    db.execute_unprepared("TRUNCATE persons CASCADE")
        .await
        .expect("truncate persons");

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
    let contents = std::fs::read_to_string(&path).expect("read the real fixture");
    let (fixture, failures) = parse_fixture(&contents);
    assert!(
        failures.is_empty(),
        "unexpected parse failures: {failures:?}"
    );
    assert_eq!(fixture.len(), 50, "fixture should hold 50 persons");

    // First run: the table is empty, so every row lands.
    let first = seed(&db, &fixture).await.expect("first seed run");
    assert_eq!(first.created, 50);
    assert_eq!(first.existing, 0);
    assert!(!first.skipped);

    let count = persons::Entity::find()
        .count(&db)
        .await
        .expect("count persons");
    assert_eq!(count, 50);

    // Both halves of the "Adaeze Okonkwo / Okonkow" duplicate pair
    // (examples/data/README.md "The duplicate pairs", fixture lines 1 &
    // 4) are present — the reason this task bypasses the create
    // endpoint's duplicate detection in the first place. `person_names`
    // is the child table `family` actually lives in.
    let pair_count: i64 = {
        let row = db
            .query_one_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT COUNT(*) AS n FROM person_names WHERE family IN ('Okonkwo', 'Okonkow')",
                [],
            ))
            .await
            .expect("duplicate-pair count query")
            .expect("one row");
        row.try_get("", "n").expect("n")
    };
    assert_eq!(
        pair_count, 2,
        "both halves of the Okonkwo/Okonkow duplicate pair must be persisted"
    );

    // Second run: the table is no longer empty, so the idempotency guard
    // must refuse to insert a second copy of every row.
    let second = seed(&db, &fixture).await.expect("second seed run");
    assert_eq!(second.created, 0);
    assert_eq!(second.existing, 50);
    assert!(second.skipped);

    let count_after = persons::Entity::find()
        .count(&db)
        .await
        .expect("count persons after second run");
    assert_eq!(count_after, 50, "a re-run must not double the table");
}
