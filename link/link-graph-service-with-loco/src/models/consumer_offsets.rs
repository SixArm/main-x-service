//! `consumer_offsets` model — per-topic bus position + freshness
//! watermark (spec §10.3 / §6 FR-16/17).

use chrono::{DateTime, FixedOffset};
use loco_rs::prelude::*;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, ConnectionTrait};

pub use super::_entities::consumer_offsets::{self, ActiveModel, Column, Entity, Model};

/// Default `SeaORM` active-model behaviour — no custom hooks.
impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// Record the latest position + freshness watermark for `topic`.
    /// Idempotent on the topic primary key.
    ///
    /// # Errors
    ///
    /// When the upsert fails.
    pub async fn record<C: ConnectionTrait>(
        db: &C,
        topic: &str,
        offset: i64,
        occurred_at: DateTime<FixedOffset>,
    ) -> ModelResult<()> {
        let am = consumer_offsets::ActiveModel {
            topic: ActiveValue::set(topic.to_string()),
            offset_val: ActiveValue::set(offset),
            last_occurred_at: ActiveValue::set(occurred_at),
        };
        Entity::insert(am)
            .on_conflict(
                OnConflict::column(Column::Topic)
                    .update_columns([Column::OffsetVal, Column::LastOccurredAt])
                    .to_owned(),
            )
            .exec(db)
            .await?;
        Ok(())
    }

    /// Per-topic freshness: `(topic, last_occurred_at)` for every topic.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn freshness<C: ConnectionTrait>(
        db: &C,
    ) -> ModelResult<Vec<(String, DateTime<FixedOffset>)>> {
        let rows = Entity::find().all(db).await?;
        Ok(rows
            .into_iter()
            .map(|m| (m.topic, m.last_occurred_at))
            .collect())
    }

    /// The read-model freshness watermark = the newest `last_occurred_at`
    /// across all topics. This is the `as_of` stamped on graph responses
    /// (spec §6 FR-17). `None` when nothing has been consumed yet.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn watermark<C: ConnectionTrait>(
        db: &C,
    ) -> ModelResult<Option<DateTime<FixedOffset>>> {
        let latest = Self::freshness(db).await?.into_iter().map(|(_, t)| t).max();
        Ok(latest)
    }
}
