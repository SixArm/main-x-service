//! DB-gated integration tests for the durable event bus Phase-2
//! transactional outbox (`agents/share/event-bus.md` §3, §5).
//!
//! These boot the real loco app against the `test` environment config, so
//! they require a reachable PostgreSQL instance and are therefore
//! `#[ignore]`d (run with `cargo test -- --ignored`). They exercise the
//! exact write path the `outbox` transport takes in the CRUD handlers:
//! one transaction spanning the `plans` insert **and** the
//! `event_outbox` insert, so the two commit or roll back together.
//!
//! The pure envelope→row mapping and the transport-string parsing are
//! pinned DB-free in `src/models/event_outbox.rs` and `src/streaming.rs`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_matcher::Plan;
use project_portfolio_management_service::app::App;
use project_portfolio_management_service::models::_entities::{event_outbox, plans};
use project_portfolio_management_service::models::event_outbox::OutboxInsert;
use project_portfolio_management_service::models::plans::Model as PlanModel;
use project_portfolio_management_service::streaming::{
    ENTITY, Envelope, EventKind, SCHEMA_VERSION,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, TransactionTrait};
use serial_test::serial;
use uuid::Uuid;

/// Build a `Created` envelope for a freshly-created plan row.
fn created_envelope(model: &PlanModel) -> Envelope {
    Envelope {
        event_id: Uuid::new_v4(),
        schema_version: SCHEMA_VERSION,
        entity: ENTITY,
        kind: EventKind::Created,
        pid: model.pid.to_string(),
        seq: 1,
        actor: None,
        name: model.name.clone(),
    }
}

/// Outbox happy path: creating a plan writes BOTH the plan row
/// and exactly one `event_outbox` row, in one transaction. This mirrors
/// the `outbox`-transport CRUD handler path (`streaming::create_and_emit`).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn outbox_create_writes_plan_and_one_event_atomically() {
    super::isolate_search_index();
    request::<App, _, _>(|_request, ctx| async move {
        let wi = Plan::new("Outbox platform migration");

        // One transaction spanning the entity insert and the outbox insert.
        let txn = ctx.db.begin().await.expect("begin tx");
        let model = PlanModel::create(&txn, &wi).await.expect("create plan");
        let env = created_envelope(&model);
        OutboxInsert::from_envelope(&env, chrono::Utc::now().into())
            .expect("map envelope")
            .insert_on(&txn)
            .await
            .expect("enqueue outbox row");
        txn.commit().await.expect("commit tx");

        // The plan row persisted...
        let wi_count = plans::Entity::find()
            .count(&ctx.db)
            .await
            .expect("count plans");
        assert_eq!(wi_count, 1, "the plans row committed");

        // ...and exactly one outbox row for it, unpublished, correctly mapped.
        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(model.pid))
            .all(&ctx.db)
            .await
            .expect("load outbox rows");
        assert_eq!(rows.len(), 1, "exactly one event_outbox row for the plan");
        assert_eq!(rows[0].kind, "created");
        assert_eq!(rows[0].entity, "plan");
        assert_eq!(rows[0].event_id, env.event_id);
        assert!(
            rows[0].published_at.is_none(),
            "the row starts unpublished (the relay stamps it)"
        );
    })
    .await;
}

/// Outbox atomicity: a failure before commit rolls back BOTH the
/// plan row and its `event_outbox` row — neither is left behind.
/// This is the crash window the outbox closes (no committed change without
/// its event, and no event without its committed change).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn outbox_failure_rolls_back_both_writes() {
    super::isolate_search_index();
    request::<App, _, _>(|_request, ctx| async move {
        let wi = Plan::new("Rollback platform migration");

        let txn = ctx.db.begin().await.expect("begin tx");
        let model = PlanModel::create(&txn, &wi).await.expect("create plan");
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
        let wi_count = plans::Entity::find()
            .count(&ctx.db)
            .await
            .expect("count plans");
        assert_eq!(wi_count, 0, "the plans insert rolled back");
        let event_count = event_outbox::Entity::find()
            .count(&ctx.db)
            .await
            .expect("count outbox rows");
        assert_eq!(event_count, 0, "the event insert rolled back with it");
    })
    .await;
}
