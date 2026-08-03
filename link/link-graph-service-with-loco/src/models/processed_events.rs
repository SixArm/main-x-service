//! `processed_events` model — bus-consumer idempotency (spec §10.3;
//! BUS-2). Under at-least-once delivery a redelivered event must not
//! re-apply; [`events::apply_event_idempotent`](crate::events::apply_event_idempotent)
//! checks [`Model::is_processed`] before folding an event into the
//! read-model and calls [`Model::mark_processed`] after a successful
//! apply.

use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, ConnectionTrait, EntityTrait};
use uuid::Uuid;

pub use super::_entities::processed_events::{self, ActiveModel, Column, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Whether `event_id` has already been applied to the read-model.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn is_processed<C: ConnectionTrait>(db: &C, event_id: Uuid) -> ModelResult<bool> {
        Ok(Entity::find_by_id(event_id).one(db).await?.is_some())
    }

    /// Record `event_id` as processed. Idempotent on the primary key — a
    /// duplicate record (a second successful apply of an already-recorded
    /// event, which [`is_processed`](Model::is_processed) should normally
    /// prevent, but which a race between two consumers of the same
    /// redelivered event could still produce) is a no-op, not an error.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn mark_processed<C: ConnectionTrait>(db: &C, event_id: Uuid) -> ModelResult<()> {
        let am = processed_events::ActiveModel {
            event_id: ActiveValue::set(event_id),
            processed_at: ActiveValue::set(Utc::now().fixed_offset()),
        };
        Entity::insert(am)
            .on_conflict(OnConflict::column(Column::EventId).do_nothing().to_owned())
            .exec(db)
            .await?;
        Ok(())
    }

    /// Delete processed-event rows older than `retention_days` (a
    /// short-lived dedup window, not durable history — durability of the
    /// change feed is Fluvio's job, `agents/share/event-bus.md` §3).
    /// Returns the number of rows purged.
    ///
    /// # Errors
    ///
    /// When the delete query fails.
    pub async fn purge_older_than<C: ConnectionTrait>(
        db: &C,
        retention_days: i64,
    ) -> ModelResult<u64> {
        use sea_orm::{ColumnTrait, QueryFilter};
        let cutoff =
            Utc::now() - chrono::Duration::try_days(retention_days.max(0)).unwrap_or_default();
        let res = Entity::delete_many()
            .filter(Column::ProcessedAt.lt(cutoff.fixed_offset()))
            .exec(db)
            .await?;
        Ok(res.rows_affected)
    }
}
