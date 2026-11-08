//! `red_green_days` — one row per stay per calendar day: red/green
//! classification + at most two coded delay reasons.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ModelResult;

/// One `red_green_days` row, serialized as-is in the stay detail and the
/// `red-green` response (the wire shape the `SeaORM` entity had).
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub stay_pid: Uuid,
    pub day: NaiveDate,
    pub classification: String,
    pub delay_reasons: serde_json::Value,
    pub note: Option<String>,
}

impl Model {
    /// One stay's journal, in day order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_stay(db: impl Into<Db<'_>>, stay_pid: Uuid) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, stay_pid, day, classification, delay_reasons, note
               FROM red_green_days
               WHERE stay_pid = $1
               ORDER BY day"#,
            stay_pid,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// The entries for `day` of any of these stays (the whiteboard).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_stays_on(
        db: impl Into<Db<'_>>,
        stay_pids: &[Uuid],
        day: NaiveDate,
    ) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, stay_pid, day, classification, delay_reasons, note
               FROM red_green_days
               WHERE stay_pid = ANY($1) AND day = $2"#,
            stay_pids,
            day,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Record `day` for this stay, or rewrite it if the stay already has
    /// an entry for that day, returning the row (PF-R1).
    ///
    /// One statement on the `red_green_stay_day` key, not a read then an
    /// insert: two concurrent same-day POSTs each saw no row, and the
    /// second insert then broke the unique index and answered `409`.
    /// Here the second writer waits for the first to commit and updates
    /// the day instead, so the later write wins, as a same-day edit does.
    /// `created_at` keeps the first write's time; `updated_at` moves.
    ///
    /// # Errors
    ///
    /// When the statement fails.
    pub async fn upsert(
        db: impl Into<Db<'_>>,
        stay_pid: Uuid,
        day: NaiveDate,
        classification: &str,
        delay_reasons: serde_json::Value,
        note: Option<&str>,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO red_green_days (stay_pid, day, classification, delay_reasons, note)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (stay_pid, day) DO UPDATE
               SET updated_at = now(),
                   classification = EXCLUDED.classification,
                   delay_reasons = EXCLUDED.delay_reasons,
                   note = EXCLUDED.note
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, stay_pid, day, classification, delay_reasons, note"#,
            stay_pid,
            day,
            classification,
            delay_reasons,
            note,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }
}
