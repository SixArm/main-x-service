//! DB-gated proof for the `seed_examples` task (EX-4,
//! `src/tasks/seed_examples.rs`): a first run seeds every fixture row,
//! and a second run is a no-op rather than doubling the table.
//!
//! Owns the whole `organizations` table for its assertions (row
//! counts), so it truncates at the start.
//!
//! `#[ignore]`d: boots the app, so it needs PostgreSQL via
//! `config/test.yaml`. Run with:
//! `cargo test --test seed_examples_db -- --ignored`
//! (or `scripts/ci-check.sh test-db organization/organization-service-with-loco`).

use std::path::Path;

use loco_rs::testing::prelude::*;
use organization_service::app::App;
use organization_service::models::_entities::organizations;
use organization_service::tasks::seed_examples::{FIXTURE_PATH, parse_fixture, seed};
use sea_orm::{ConnectionTrait, EntityTrait, PaginatorTrait};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test --test seed_examples_db -- --ignored"]
async fn seed_examples_seeds_the_fixture_and_is_idempotent() {
    request::<App, _, _>(|_request, ctx| async move {
        // Clean slate: this test asserts on the whole table's row
        // count, so it needs sole ownership of the starting state
        // regardless of what ran before it in the shared test database.
        ctx.db
            .execute_unprepared("TRUNCATE organizations CASCADE")
            .await
            .expect("truncate organizations");

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
        let contents = std::fs::read_to_string(&path).expect("read the real fixture");
        let (fixture, failures) = parse_fixture(&contents);
        assert!(
            failures.is_empty(),
            "unexpected parse failures: {failures:?}"
        );
        assert_eq!(fixture.len(), 20, "fixture should hold 20 organizations");

        // First run: the table is empty, so every row lands.
        let first = seed(&ctx.db, &fixture).await.expect("first seed run");
        assert_eq!(first.created, 20);
        assert_eq!(first.existing, 0);
        assert!(!first.skipped);

        let count = organizations::Entity::find()
            .count(&ctx.db)
            .await
            .expect("count organizations");
        assert_eq!(count, 20);

        // Second run: the table is no longer empty, so the idempotency
        // guard must refuse to insert a second copy of every row.
        let second = seed(&ctx.db, &fixture).await.expect("second seed run");
        assert_eq!(second.created, 0);
        assert_eq!(second.existing, 20);
        assert!(second.skipped);

        let count_after = organizations::Entity::find()
            .count(&ctx.db)
            .await
            .expect("count organizations after second run");
        assert_eq!(count_after, 20, "a re-run must not double the table");
    })
    .await;
}
