//! DB-gated proof that `GET /api/compliance/audit/verify` examines the
//! **newest** audit rows (LG-A1), not an arbitrary sample.
//!
//! `audit_log.id` is a random UUID, so ordering by it picked whichever
//! rows happened to sort last. This test makes the newest row sort first
//! by id — any UUID is a possible random draw — tampers with it in SQL,
//! and checks that a small `limit` still reports it.
//!
//! `#[ignore]`d so the default `cargo test` stays green on a
//! database-less machine:
//!
//! ```sh
//! DATABASE_URL=postgres://loco:loco@localhost:5432/link_graph_service_test \
//!   cargo test --test audit_verify -- --ignored
//! ```

use axa::testing::prelude::*;
use link_graph_service::app::App;
use link_graph_service::models::audit_log::{self, AuditContext};
use serde_json::Value;
use serial_test::serial;
use uuid::Uuid;

/// More rows than the `limit` the test asks for, so a wrong order can
/// leave the newest row out.
const ROWS: usize = 12;
const LIMIT: usize = 3;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL; run with `cargo test --test audit_verify -- --ignored`"]
async fn verify_examines_the_newest_rows_and_catches_a_tampered_newest_row() {
    request::<App, _, _>(|request, ctx| async move {
        let audit = AuditContext {
            actor: Some("auditor"),
            user_ip: None,
            user_agent: None,
        };
        for i in 0..ROWS {
            audit_log::Model::record(
                &ctx.db,
                &audit,
                "read_edge",
                "subject_of",
                &format!("case:{}", Uuid::from_u128(1000 + i as u128)),
                &format!("person:{}", Uuid::from_u128(2000 + i as u128)),
            )
            .await
            .expect("record audit row");
        }

        // The newest row gets the smallest possible id, so `ORDER BY id
        // DESC` sorts it last. `id` is outside the digest pre-image, so
        // this alone leaves the row intact.
        let newest = Uuid::nil();
        let moved = sqlx::query(
            "UPDATE audit_log SET id = $1
             WHERE id = (SELECT id FROM audit_log
                         ORDER BY occurred_at DESC, id DESC LIMIT 1)",
        )
        .bind(newest)
        .execute(&ctx.db)
        .await
        .expect("move newest id");
        assert_eq!(moved.rows_affected(), 1);

        let before: Value = request
            .get(&format!("/api/compliance/audit/verify?limit={LIMIT}"))
            .await
            .json();
        assert_eq!(before["verified"], true, "untampered trail: {before}");
        assert_eq!(before["rows"], LIMIT);

        // Tamper with the newest row's content.
        sqlx::query("UPDATE audit_log SET action = 'tampered' WHERE id = $1")
            .bind(newest)
            .execute(&ctx.db)
            .await
            .expect("tamper");

        let after: Value = request
            .get(&format!("/api/compliance/audit/verify?limit={LIMIT}"))
            .await
            .json();
        assert_eq!(after["rows"], LIMIT);
        assert_eq!(
            after["verified"], false,
            "a tampered newest row must be examined and caught: {after}"
        );
        assert_eq!(after["mismatched"], serde_json::json!([newest.to_string()]));
    })
    .await;
}
