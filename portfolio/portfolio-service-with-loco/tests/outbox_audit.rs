//! DB-gated proof that under the `outbox` transport, `create_and_emit`
//! writes the entity, its `event_outbox` row, **and** its `audit_logs`
//! row in **one transaction** (`agents/share/event-bus.md` §3: audit and
//! outbox share the handler transaction, so they can never disagree).
//!
//! This drives the real `streaming::create_and_emit` path (not a
//! hand-built transaction), so it also exercises the transport switch. It
//! runs in its **own test binary** so the process-wide `transport()`
//! `OnceLock` — pinned to `outbox` here — does not leak into the
//! enforcement-off request suite (which expects the default `memory`).
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test outbox_audit -- --ignored`.

use loco_rs::testing::prelude::*;
use portfolio_matcher::{WorkItem, WorkItemKind};
use portfolio_service::app::App;
use portfolio_service::models::_entities::{audit_logs, event_outbox, work_items};
use portfolio_service::streaming;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test --test outbox_audit -- --ignored"]
async fn outbox_create_and_emit_writes_entity_event_and_audit_atomically() {
    // Select the durable transport BEFORE the app boots (the transport is
    // read once into a process-wide OnceLock). `set_var` is `unsafe` in
    // edition 2024; single-threaded test setup.
    unsafe {
        std::env::set_var("PORTFOLIO_EVENT_TRANSPORT", "outbox");
    }

    request::<App, _, _>(|_request, ctx| async move {
        let actor = "11111111-1111-1111-1111-111111111111";
        let wi = WorkItem::new(WorkItemKind::Portfolio, "Outbox Audit Portfolio");

        // The real handler path: entity + event + audit, one transaction.
        let model = streaming::create_and_emit(&ctx.db, "Portfolio", &wi, Some(actor))
            .await
            .expect("create_and_emit under outbox transport");

        // Entity committed.
        assert_eq!(
            work_items::Entity::find()
                .filter(work_items::Column::Pid.eq(model.pid))
                .count(&ctx.db)
                .await
                .expect("count work items"),
            1,
            "the work_items row committed"
        );

        // Exactly one outbox row, unpublished, for this record.
        let events = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(model.pid))
            .all(&ctx.db)
            .await
            .expect("load outbox rows");
        assert_eq!(events.len(), 1, "one event_outbox row");
        assert_eq!(events[0].kind, "created");
        assert!(events[0].published_at.is_none());

        // ...and the audit row rode the SAME transaction (§3) — not a
        // best-effort after-commit side channel.
        let audits = audit_logs::Entity::find()
            .filter(audit_logs::Column::EntityPid.eq(model.pid))
            .all(&ctx.db)
            .await
            .expect("load audit rows");
        assert_eq!(audits.len(), 1, "one audit_logs row, atomic with the event");
        assert_eq!(audits[0].action, "created");
        assert_eq!(audits[0].actor.as_deref(), Some(actor));
    })
    .await;
}
