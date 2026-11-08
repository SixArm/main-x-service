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

    /// One stay's entry for `day`, if recorded.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_stay_on(
        db: impl Into<Db<'_>>,
        stay_pid: Uuid,
        day: NaiveDate,
    ) -> ModelResult<Option<Self>> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, stay_pid, day, classification, delay_reasons, note
               FROM red_green_days
               WHERE stay_pid = $1 AND day = $2
               LIMIT 1"#,
            stay_pid,
            day,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
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

    /// Record a new day, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails (a second row for the same stay and day is
    /// a unique violation).
    pub async fn insert(
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

    /// Rewrite the same-day entry with this `id`, returning it.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn reclassify(
        db: impl Into<Db<'_>>,
        id: i64,
        classification: &str,
        delay_reasons: serde_json::Value,
        note: Option<&str>,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE red_green_days SET classification = $2, delay_reasons = $3, note = $4
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, stay_pid, day, classification, delay_reasons, note"#,
            id,
            classification,
            delay_reasons,
            note,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }
}
