//! DB-gated integration tests for the durable event bus Phase-2
//! transactional outbox (`agents/share/event-bus.md` §3, §5).
//!
//! These boot the real loco app against the `test` environment config, so
//! they require a reachable PostgreSQL instance and are therefore
//! `#[ignore]`d (run with `cargo test -- --ignored`). They exercise the
//! exact write path the `outbox` transport takes in the CRUD handlers:
//! one transaction spanning the `cases` insert **and** the `event_outbox`
//! insert, so the two commit or roll back together.
//!
//! The pure envelope→row mapping and the transport-string parsing are
//! pinned DB-free in `src/models/event_outbox.rs` and `src/streaming.rs`.

use case_matcher::Case;
use case_service::app::App;
use case_service::models::_entities::{cases, event_outbox};
use case_service::models::cases::Model as CaseModel;
use case_service::models::event_outbox::OutboxInsert;
use case_service::streaming::{ENTITY, Envelope, EventKind, SCHEMA_VERSION};
use loco_rs::testing::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, TransactionTrait};
use serial_test::serial;
use uuid::Uuid;

/// Build a `Created` envelope for a freshly-created case row.
fn created_envelope(model: &CaseModel) -> Envelope {
    Envelope {
        event_id: Uuid::new_v4(),
        schema_version: SCHEMA_VERSION,
        entity: ENTITY,
        kind: EventKind::Created,
        pid: model.pid.to_string(),
        seq: 1,
        actor: None,
        name: model.title.clone(),
        data: None,
    }
}

/// Outbox happy path: creating a case writes BOTH the case row and exactly
/// one `event_outbox` row, in one transaction. This mirrors the
/// `outbox`-transport CRUD handler path (`streaming::create_and_emit`).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn outbox_create_writes_case_and_one_event_atomically() {
    request::<App, _, _>(|_request, ctx| async move {
        let case = Case::new("Housing benefit appeal");

        // One transaction spanning the entity insert and the outbox insert.
        let txn = ctx.db.begin().await.expect("begin tx");
        let model = CaseModel::create(&txn, &case).await.expect("create case");
        let env = created_envelope(&model);
        OutboxInsert::from_envelope(&env, chrono::Utc::now().into())
            .expect("map envelope")
            .insert_on(&txn)
            .await
            .expect("enqueue outbox row");
        txn.commit().await.expect("commit tx");

        // The case row persisted...
        let case_count = cases::Entity::find()
            .count(&ctx.db)
            .await
            .expect("count cases");
        assert_eq!(case_count, 1, "the case row committed");

        // ...and exactly one outbox row for it, unpublished, correctly mapped.
        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(model.pid))
            .all(&ctx.db)
            .await
            .expect("load outbox rows");
        assert_eq!(rows.len(), 1, "exactly one event_outbox row for the case");
        assert_eq!(rows[0].kind, "created");
        assert_eq!(rows[0].entity, "case");
        assert_eq!(rows[0].event_id, env.event_id);
        assert!(
            rows[0].published_at.is_none(),
            "the row starts unpublished (the relay stamps it)"
        );
    })
    .await;
}

/// Outbox atomicity: a failure before commit rolls back BOTH the case row
/// and its `event_outbox` row — neither is left behind. This is the crash
/// window the outbox closes (no committed change without its event, and no
/// event without its committed change).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn outbox_failure_rolls_back_both_writes() {
    request::<App, _, _>(|_request, ctx| async move {
        let case = Case::new("Tax credit overpayment");

        let txn = ctx.db.begin().await.expect("begin tx");
        let model = CaseModel::create(&txn, &case).await.expect("create case");
        let env = created_envelope(&model);
        OutboxInsert::from_envelope(&env, chrono::Utc::now().into())
            .expect("map envelope")
            .insert_on(&txn)
            .await
            .expect("enqueue outbox row");
        // Simulate a handler error after both writes but before commit:
        // roll the whole transaction back.
        txn.rollback().await.expect("rollback tx");

        // Neither write survived — the two are all-or-nothing.
        let case_count = cases::Entity::find()
            .count(&ctx.db)
            .await
            .expect("count cases");
        assert_eq!(case_count, 0, "the case insert rolled back");
        let event_count = event_outbox::Entity::find()
            .count(&ctx.db)
            .await
            .expect("count outbox rows");
        assert_eq!(event_count, 0, "the event insert rolled back with it");
    })
    .await;
}
