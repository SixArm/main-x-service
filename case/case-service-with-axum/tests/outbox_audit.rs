//! DB-gated proof that under the `outbox` transport, `create_and_emit`
//! writes the entity, its `event_outbox` row, **and** its `audit_logs`
//! row in **one transaction** (`agents/share/event-bus.md` §3: audit and
//! outbox share the handler transaction, so they can never disagree).
//!
//! This drives the real `streaming::create_and_emit` path (not a
//! hand-built transaction like `tests/requests/event_outbox.rs`), so it
//! also exercises the transport switch. It runs in its **own test binary**
//! so the process-wide `transport()` `OnceLock` — pinned to `outbox` here
//! — does not leak into the enforcement-off request suite (which expects
//! the default `memory`).
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test outbox_audit -- --ignored`.

use axa::testing::prelude::*;
use case_matcher::Case;
use case_service::app::App;
use case_service::models::{audit_logs, cases, event_outbox};
use case_service::streaming;
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test --test outbox_audit -- --ignored"]
async fn outbox_create_and_emit_writes_entity_event_and_audit_atomically() {
    // Select the durable transport BEFORE the app boots (the transport is
    // read once into a process-wide OnceLock). `set_var` is `unsafe` in
    // edition 2024; single-threaded test setup.
    unsafe {
        // Keep this binary's Tantivy index out of the working directory
        // and its boot rebuild out of the way (see tests/requests/mod.rs).
        std::env::set_var(
            "CASE_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!("case-outbox-audit-index-{}", std::process::id())),
        );
        std::env::set_var("CASE_SEARCH_BOOT_REINDEX", "0");
        std::env::set_var("CASE_EVENT_TRANSPORT", "outbox");
    }

    request::<App, _, _>(|_request, ctx| async move {
        let actor = "11111111-1111-1111-1111-111111111111";
        let case = Case::new("Outbox Audit Case");

        // The real handler path: entity + event + audit, one transaction.
        let model = streaming::create_and_emit(&ctx.db, &case, Some(actor))
            .await
            .expect("create_and_emit under outbox transport");

        // Entity committed.
        assert_eq!(
            cases::Model::count_by_pid(&ctx.db, model.pid)
                .await
                .expect("count cases"),
            1,
            "the case row committed"
        );

        // Exactly one outbox row, unpublished, for this record.
        let events = event_outbox::Model::for_entity(&ctx.db, model.pid)
            .await
            .expect("load outbox rows");
        assert_eq!(events.len(), 1, "one event_outbox row");
        assert_eq!(events[0].kind, "created");
        assert!(events[0].published_at.is_none());

        // ...and the audit row rode the SAME transaction (§3) — not a
        // best-effort after-commit side channel.
        let audits = audit_logs::Model::for_entity(&ctx.db, model.pid)
            .await
            .expect("load audit rows");
        assert_eq!(audits.len(), 1, "one audit_logs row, atomic with the event");
        assert_eq!(audits[0].action, "created");
        assert_eq!(audits[0].actor.as_deref(), Some(actor));
    })
    .await;
}
