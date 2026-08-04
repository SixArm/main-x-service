//! DB-gated proof that [`suggestion_runs::Model::record`] durably persists
//! one row per completed cross-service `same_identity` suggestion pass
//! (T-33, design §16 OQ-9(d) "audit ... every run's counts") — not merely
//! that the migration applies cleanly (already exercised implicitly by
//! every other DB-gated test in this crate booting the app), but that a
//! recorded pass's counts round-trip through Postgres exactly.
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test suggestion_runs -- --ignored`.

use chrono::Utc;
use link_graph_service::app::App;
use link_graph_service::models::_entities::suggestion_runs;
use link_graph_service::models::suggestion_runs::{Model, SuggestionRunRecord};
use loco_rs::testing::prelude::*;
use sea_orm::EntityTrait;
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test suggestion_runs -- --ignored`"]
async fn record_durably_persists_one_row_per_completed_pass() {
    request::<App, _, _>(|_request, ctx| async move {
        let started_at = Utc::now().fixed_offset();
        let rec = SuggestionRunRecord {
            started_at,
            persons_fetched: 25,
            workers_fetched: 21,
            candidates: 3,
            posted: 2,
            failed: 0,
            dropped: 1,
            max_candidates: 50,
            max_edges_per_run: 2,
        };
        Model::record(&ctx.db, &rec)
            .await
            .expect("recording a completed pass succeeds");

        let rows = suggestion_runs::Entity::find()
            .all(&ctx.db)
            .await
            .expect("query succeeds");
        assert_eq!(rows.len(), 1, "exactly one durable row per completed pass");
        let row = &rows[0];
        assert_eq!(row.persons_fetched, 25);
        assert_eq!(row.workers_fetched, 21);
        assert_eq!(row.candidates, 3);
        assert_eq!(row.posted, 2);
        assert_eq!(row.failed, 0);
        assert_eq!(row.dropped, 1);
        assert_eq!(row.max_candidates, 50);
        assert_eq!(row.max_edges_per_run, 2);
        assert!(
            row.completed_at >= row.started_at,
            "completed_at must not precede started_at"
        );

        // A second completed pass adds a second row — this is a durable
        // history, not a single last-known-value slot (that is what the
        // `link_graph_suggestion_last_run` gauge is for; this table is
        // deliberately not it).
        let rec2 = SuggestionRunRecord {
            started_at: Utc::now().fixed_offset(),
            persons_fetched: 26,
            workers_fetched: 21,
            candidates: 0,
            posted: 0,
            failed: 0,
            dropped: 0,
            max_candidates: 50,
            max_edges_per_run: 200,
        };
        Model::record(&ctx.db, &rec2)
            .await
            .expect("recording a second completed pass succeeds");
        let rows_after = suggestion_runs::Entity::find()
            .all(&ctx.db)
            .await
            .expect("query succeeds");
        assert_eq!(rows_after.len(), 2, "history accumulates, not overwrites");
    })
    .await;
}
