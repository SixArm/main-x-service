//! `processed_events` model — bus-consumer idempotency (spec §10.3;
//! BUS-2). Under at-least-once delivery a redelivered event must not
//! re-apply; [`events::apply_event_idempotent`](crate::events::apply_event_idempotent)
//! checks [`Model::is_processed`] before folding an event into the
//! read-model and calls [`Model::mark_processed`] after a successful
//! apply.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ModelResult;

/// A recorded envelope `event_id` that has already been applied to the
/// read-model.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// The envelope `event_id` — the idempotency key.
    pub event_id: Uuid,
    /// When this event was recorded as processed.
    pub processed_at: DateTime<FixedOffset>,
}

impl Model {
    /// Whether `event_id` has already been applied to the read-model.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn is_processed(db: impl Into<Db<'_>>, event_id: Uuid) -> ModelResult<bool> {
        let mut conn = db.into().acquire().await?;
        let found = sqlx::query_scalar!(
            "SELECT event_id FROM processed_events WHERE event_id = $1",
            event_id,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(found.is_some())
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
    pub async fn mark_processed(db: impl Into<Db<'_>>, event_id: Uuid) -> ModelResult<()> {
        let mut conn = db.into().acquire().await?;
        let processed_at: DateTime<FixedOffset> = Utc::now().fixed_offset();
        sqlx::query!(
            "INSERT INTO processed_events (event_id, processed_at)
             VALUES ($1, $2)
             ON CONFLICT (event_id) DO NOTHING",
            event_id,
            processed_at,
        )
        .execute(&mut *conn)
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
    pub async fn purge_older_than(db: impl Into<Db<'_>>, retention_days: i64) -> ModelResult<u64> {
        let mut conn = db.into().acquire().await?;
        let cutoff =
            Utc::now() - chrono::Duration::try_days(retention_days.max(0)).unwrap_or_default();
        let cutoff: DateTime<FixedOffset> = cutoff.fixed_offset();
        let res = sqlx::query!(
            "DELETE FROM processed_events WHERE processed_at < $1",
            cutoff,
        )
        .execute(&mut *conn)
        .await?;
        Ok(res.rows_affected())
    }
}
