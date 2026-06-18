//! `merge_records` model — record and query the record-merge history.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use uuid::Uuid;

/// Re-export the generated entity types so callers use a single
/// `models::merge_records::*` path for the entity and its helpers.
pub use super::_entities::merge_records::{self, ActiveModel, Entity, Model};

/// Default active-model lifecycle hooks (merge rows are plain inserts).
impl ActiveModelBehavior for super::_entities::merge_records::ActiveModel {}

/// Recording and query helpers for the merge history.
impl Model {
    /// Record one merge. `transferred` is a snapshot of the merged-away
    /// duplicate's payload. `actor` is the caller's `sub` when a verified
    /// token was presented, else `None`.
    ///
    /// - `main_pid`: the surviving organization (kept).
    /// - `duplicate_pid`: the merged-away organization (now soft-deleted).
    /// - `reason`: optional operator-supplied free text.
    /// - `actor`: verified caller `sub`, or `None` when unauthenticated.
    /// - `transferred`: the duplicate's payload snapshot for recovery.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(
        db: &DatabaseConnection,
        main_pid: Uuid,
        duplicate_pid: Uuid,
        reason: Option<&str>,
        actor: Option<&str>,
        transferred: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        let entry = merge_records::ActiveModel {
            main_pid: ActiveValue::set(main_pid),
            duplicate_pid: ActiveValue::set(duplicate_pid),
            reason: ActiveValue::set(reason.map(ToString::to_string)),
            actor: ActiveValue::set(actor.map(ToString::to_string)),
            transferred: ActiveValue::set(transferred),
            // `id`, `created_at`, `updated_at` are DB/SeaORM-managed.
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(entry)
    }

    /// Most-recent merge records, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = merge_records::Entity::find()
            .order_by_desc(merge_records::Column::Id)
            .limit(limit)
            .all(db)
            .await?;
        Ok(rows)
    }
}
