//! `merge_records` model — record and query the record-merge history.

use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::merge_records::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::merge_records::ActiveModel {}

impl Model {
    /// Record one merge. `actor` is the caller's `sub` when a verified
    /// token was presented; `transferred` is a snapshot of the merged-away
    /// duplicate's payload.
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
