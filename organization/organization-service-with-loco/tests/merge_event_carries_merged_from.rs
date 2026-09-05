//! DB-gated proof that `streaming::merge_and_emit`'s survivor `Merged`
//! event carries the absorbed duplicate's pid in `merged_from` (spec
//! `spec/index.md` §13, umbrella spec §13 T-13). Without this, the
//! link-graph aggregator's merge-repointing consumer
//! (`agents/share/cross-service-linking.md` §5.3) has no way to know
//! which edges to move off the duplicate onto the survivor.
//!
//! Driven under the `outbox` transport, whose `event_outbox` row
//! persists the **full** serialized `Envelope` as its `payload` — the
//! only place a merge event's complete shape can be inspected after the
//! fact. (The `memory` transport's `/events/recent` projection is the
//! frozen `EventView` shape, which deliberately does not carry
//! `merged_from`; that construction is pinned DB-free instead, in
//! `src/streaming.rs`'s `merge_envelope_carries_merged_from`.)
//!
//! This runs in its **own test binary**, same reasoning as
//! `tests/outbox_audit.rs`: the transport is a process-wide `OnceLock`
//! read once at first use, so pinning it to `outbox` here must not leak
//! into the enforcement-off request suite (which expects `memory`).
//!
//! `#[ignore]`d: boots the app against Postgres. Run with
//! `cargo test --test merge_event_carries_merged_from -- --ignored`.

use loco_rs::testing::prelude::*;
use organization_matcher::Organization;
use organization_service::app::App;
use organization_service::models::_entities::event_outbox;
use organization_service::models::organizations::Model as OrgModel;
use organization_service::streaming;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test --test merge_event_carries_merged_from -- --ignored"]
async fn outbox_transport_merged_event_carries_merged_from() {
    // Select the durable transport BEFORE the app boots (a process-wide
    // `OnceLock`), same pattern as `tests/outbox_audit.rs`.
    unsafe {
        std::env::set_var("ORGANIZATION_EVENT_TRANSPORT", "outbox");
        std::env::set_var(
            "ORGANIZATION_SEARCH_INDEX_PATH",
            std::env::temp_dir().join(format!(
                "organization-service-test-index-{}",
                std::process::id()
            )),
        );
    }

    request::<App, _, _>(|_request, ctx| async move {
        let main = OrgModel::create(&ctx.db, &Organization::new("Beta Main"))
            .await
            .expect("create main org");
        let duplicate = OrgModel::create(&ctx.db, &Organization::new("Beta Duplicate"))
            .await
            .expect("create duplicate org");
        let dup_pid = duplicate.pid;
        let merged_org = Organization::new("Beta Main");

        let (survivor, returned_dup_pid, _dup_name) =
            streaming::merge_and_emit(&ctx.db, main, duplicate, &merged_org, None)
                .await
                .expect("merge_and_emit under outbox transport");
        assert_eq!(returned_dup_pid, dup_pid);

        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(survivor.pid))
            .filter(event_outbox::Column::Kind.eq("merged"))
            .all(&ctx.db)
            .await
            .expect("load outbox rows for the survivor");
        assert_eq!(
            rows.len(),
            1,
            "one merged event_outbox row for the survivor"
        );
        assert_eq!(
            rows[0].payload["merged_from"],
            dup_pid.to_string(),
            "the survivor's Merged envelope must carry the absorbed duplicate's pid: {:?}",
            rows[0].payload
        );
    })
    .await;
}
