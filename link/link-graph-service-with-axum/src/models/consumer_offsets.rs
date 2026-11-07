//! `consumer_offsets` model — per-topic bus position + freshness
//! watermark (spec §10.3 / §6 FR-16/17).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use super::ModelResult;

/// A persisted per-topic offset + freshness watermark row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// The bus topic (`mxi.<entity>.events`).
    pub topic: String,
    /// The last committed bus offset for this topic.
    pub offset_val: i64,
    /// The `occurred_at` of the last consumed event on this topic.
    pub last_occurred_at: DateTime<FixedOffset>,
}

impl Model {
    /// Record the latest position + freshness watermark for `topic`.
    /// Idempotent on the topic primary key.
    ///
    /// # Errors
    ///
    /// When the upsert fails.
    pub async fn record(
        db: impl Into<Db<'_>>,
        topic: &str,
        offset: i64,
        occurred_at: DateTime<FixedOffset>,
    ) -> ModelResult<()> {
        let mut conn = db.into().acquire().await?;
        sqlx::query!(
            "INSERT INTO consumer_offsets (topic, offset_val, last_occurred_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (topic) DO UPDATE
                 SET offset_val = excluded.offset_val,
                     last_occurred_at = excluded.last_occurred_at",
            topic,
            offset,
            occurred_at,
        )
        .execute(&mut *conn)
        .await?;
        Ok(())
    }

    /// Per-topic freshness: `(topic, last_occurred_at)` for every topic.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn freshness(
        db: impl Into<Db<'_>>,
    ) -> ModelResult<Vec<(String, DateTime<FixedOffset>)>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT topic, offset_val, last_occurred_at AS "last_occurred_at: _"
               FROM consumer_offsets"#,
        )
        .fetch_all(&mut *conn)
        .await?;
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
    pub async fn watermark(db: impl Into<Db<'_>>) -> ModelResult<Option<DateTime<FixedOffset>>> {
        let latest = Self::freshness(db).await?.into_iter().map(|(_, t)| t).max();
        Ok(latest)
    }
}
