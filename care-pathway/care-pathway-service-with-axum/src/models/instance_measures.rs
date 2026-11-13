//! `instance_measures` model — one instance's recorded outcome measures
//! (a named numeric or text value on a date).

use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::ModelResult;

/// One recorded measure. Serialized as-is by the instance endpoints, in
/// column order, a fixed wire shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp (column default).
    pub created_at: DateTime<FixedOffset>,
    /// Row update timestamp: the column default, never bumped.
    pub updated_at: DateTime<FixedOffset>,
    /// Internal auto-increment primary key.
    pub id: i32,
    /// Public identifier.
    pub pid: Uuid,
    /// The owning instance's `pid`.
    pub instance_pid: Uuid,
    /// The measure's name.
    pub name: String,
    /// A numeric value.
    pub value_numeric: Option<f64>,
    /// A text value.
    pub value_text: Option<String>,
    /// The value's unit.
    pub unit: Option<String>,
    /// The day it was recorded.
    pub recorded_on: NaiveDate,
    /// The catalogued instrument this is a score on
    /// ([`crate::instruments`]), if any.
    pub instrument: Option<String>,
}

/// The columns a new measure is written with.
#[derive(Debug, Clone)]
pub struct NewMeasure<'a> {
    /// The owning instance's `pid`.
    pub instance_pid: Uuid,
    /// The measure's name.
    pub name: &'a str,
    /// A numeric value.
    pub value_numeric: Option<f64>,
    /// A text value.
    pub value_text: Option<&'a str>,
    /// The value's unit.
    pub unit: Option<&'a str>,
    /// The day it was recorded.
    pub recorded_on: NaiveDate,
    /// The catalogued instrument this is a score on, if any.
    pub instrument: Option<&'a str>,
}

impl Model {
    /// Insert one measure, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: &PgPool, new: &NewMeasure<'_>) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO instance_measures
                   (pid, instance_pid, name, value_numeric, value_text, unit, recorded_on,
                    instrument)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, instance_pid, name, value_numeric, value_text, unit,
                         recorded_on, instrument"#,
            Uuid::new_v4(),
            new.instance_pid,
            new.name,
            new.value_numeric,
            new.value_text,
            new.unit,
            new.recorded_on,
            new.instrument,
        )
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    /// One instance's measures, most recently recorded first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instance_newest_first(
        db: &PgPool,
        instance_pid: Uuid,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, name, value_numeric, value_text, unit,
                      recorded_on, instrument
               FROM instance_measures
               WHERE instance_pid = $1
               ORDER BY recorded_on DESC"#,
            instance_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every measure of the given instances, in no particular order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_instances(db: &PgPool, instance_pids: &[Uuid]) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, instance_pid, name, value_numeric, value_text, unit,
                      recorded_on, instrument
               FROM instance_measures
               WHERE instance_pid = ANY($1)"#,
            instance_pids,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}
