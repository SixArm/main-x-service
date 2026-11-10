//! `pathway_instances` model — a patient enrolled on a pathway template:
//! the status/urgency lifecycle, the review schedule, the closure record
//! and the time-based-analysis clock.
//!
//! Operational state, deliberately **not** part of the matcher payload:
//! an instance references the template by `pathway_pid` and the subject
//! by a `person:` URN. Every update writes only the columns it changes,
//! keyed on `id`, and a row that has gone is `RowNotFound` (`500`).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult, to_i64};

/// One enrolment row. Serialized as-is by the instance endpoints, so the
/// field order and the `DateTime<FixedOffset>` timestamps are a fixed wire
/// shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp (column default).
    pub created_at: DateTime<FixedOffset>,
    /// Row update timestamp: the column default, never bumped.
    pub updated_at: DateTime<FixedOffset>,
    /// Internal auto-increment primary key.
    pub id: i32,
    /// Public identifier.
    pub pid: Uuid,
    /// The pathway template's `pid`.
    pub pathway_pid: Uuid,
    /// The enrolled subject, a `person:<uuid>` URN.
    pub subject_ref: String,
    /// Lifecycle status (`active`, `on_hold`, `completed`, `discontinued`).
    pub status: String,
    /// Urgency level.
    pub urgency: String,
    /// The enrolment date.
    pub enrolled_on: NaiveDate,
    /// The next scheduled review, if any.
    pub next_review_on: Option<NaiveDate>,
    /// The closure date, once closed.
    pub closed_on: Option<NaiveDate>,
    /// Why the instance closed.
    pub closure_reason: Option<String>,
    /// The declared outcome, recorded at close.
    pub outcome: Option<String>,
    /// The time-based-analysis clock start.
    pub clock_start_at: Option<DateTime<FixedOffset>>,
    /// The time-based-analysis clock stop.
    pub clock_stop_at: Option<DateTime<FixedOffset>>,
    /// Soft-delete timestamp; `None` while live.
    pub deleted_at: Option<DateTime<FixedOffset>>,
}

/// The columns a new enrolment is written with. `id`, `created_at` and
/// `updated_at` are column defaults, and `deleted_at` starts `NULL`.
#[derive(Debug, Clone)]
pub struct NewInstance<'a> {
    /// The pathway template's `pid`.
    pub pathway_pid: Uuid,
    /// The enrolled subject URN.
    pub subject_ref: &'a str,
    /// Initial status.
    pub status: &'a str,
    /// Initial urgency.
    pub urgency: &'a str,
    /// Enrolment date.
    pub enrolled_on: NaiveDate,
    /// First scheduled review.
    pub next_review_on: Option<NaiveDate>,
    /// Closure date (a seeded closed journey).
    pub closed_on: Option<NaiveDate>,
    /// Closure reason.
    pub closure_reason: Option<&'a str>,
    /// Declared outcome.
    pub outcome: Option<&'a str>,
    /// Clock start.
    pub clock_start_at: Option<DateTime<FixedOffset>>,
    /// Clock stop.
    pub clock_stop_at: Option<DateTime<FixedOffset>>,
}

/// The `status = ANY(...)` set for a cohort's status lens: `open` is
/// active + on hold, `closed` is completed + discontinued, and anything
/// else (or none) is every status.
fn status_lens(status: Option<&str>) -> Option<Vec<String>> {
    let set: &[&str] = match status {
        Some("open") => &["active", "on_hold"],
        Some("closed") => &["completed", "discontinued"],
        _ => return None,
    };
    Some(set.iter().map(ToString::to_string).collect())
}

/// A whole-row `RETURNING` that matched no row.
fn gone<T>(row: Option<T>) -> ModelResult<T> {
    row.ok_or(ModelError::Sqlx(sqlx::Error::RowNotFound))
}

impl Model {
    /// Insert one enrolment, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewInstance<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO pathway_instances
                   (pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                    next_review_on, closed_on, closure_reason, outcome,
                    clock_start_at, clock_stop_at, deleted_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                         next_review_on, closed_on, closure_reason, outcome,
                         clock_start_at AS "clock_start_at: _",
                         clock_stop_at AS "clock_stop_at: _",
                         deleted_at AS "deleted_at: _""#,
            Uuid::new_v4(),
            new.pathway_pid,
            new.subject_ref,
            new.status,
            new.urgency,
            new.enrolled_on,
            new.next_review_on,
            new.closed_on,
            new.closure_reason,
            new.outcome,
            new.clock_start_at,
            new.clock_stop_at,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// One live (not soft-deleted) instance by `pid`, or `None`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_live(db: &PgPool, pid: Uuid) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                      next_review_on, closed_on, closure_reason, outcome,
                      clock_start_at AS "clock_start_at: _",
                      clock_stop_at AS "clock_stop_at: _",
                      deleted_at AS "deleted_at: _"
               FROM pathway_instances
               WHERE pid = $1 AND deleted_at IS NULL
               LIMIT 1"#,
            pid,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// Every instance with `pid`, soft-deleted ones included (the DB
    /// tests' read-back), or `None`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_any(db: &PgPool, pid: Uuid) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                      next_review_on, closed_on, closure_reason, outcome,
                      clock_start_at AS "clock_start_at: _",
                      clock_stop_at AS "clock_stop_at: _",
                      deleted_at AS "deleted_at: _"
               FROM pathway_instances
               WHERE pid = $1
               LIMIT 1"#,
            pid,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// A template's live instances, newest first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_pathway_newest_first(
        db: &PgPool,
        pathway_pid: Uuid,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                      next_review_on, closed_on, closure_reason, outcome,
                      clock_start_at AS "clock_start_at: _",
                      clock_stop_at AS "clock_stop_at: _",
                      deleted_at AS "deleted_at: _"
               FROM pathway_instances
               WHERE pathway_pid = $1 AND deleted_at IS NULL
               ORDER BY id DESC"#,
            pathway_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Every instance ever enrolled on a template, soft-deleted ones
    /// included, in no particular order (the seed task's read-back).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_pathway_any(db: &PgPool, pathway_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                      next_review_on, closed_on, closure_reason, outcome,
                      clock_start_at AS "clock_start_at: _",
                      clock_stop_at AS "clock_stop_at: _",
                      deleted_at AS "deleted_at: _"
               FROM pathway_instances
               WHERE pathway_pid = $1"#,
            pathway_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// The open (active + on hold) live instances, in no particular order.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn open(db: &PgPool) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                      next_review_on, closed_on, closure_reason, outcome,
                      clock_start_at AS "clock_start_at: _",
                      clock_stop_at AS "clock_stop_at: _",
                      deleted_at AS "deleted_at: _"
               FROM pathway_instances
               WHERE deleted_at IS NULL AND status IN ('active', 'on_hold')"#,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// A template's live instances under a status lens (`open`, `closed`,
    /// or anything else for all), in no particular order, capped at
    /// `limit` when one is given.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn cohort(
        db: &PgPool,
        pathway_pid: Uuid,
        status: Option<&str>,
        limit: Option<u64>,
    ) -> ModelResult<Vec<Self>> {
        let lens = status_lens(status);
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                      next_review_on, closed_on, closure_reason, outcome,
                      clock_start_at AS "clock_start_at: _",
                      clock_stop_at AS "clock_stop_at: _",
                      deleted_at AS "deleted_at: _"
               FROM pathway_instances
               WHERE pathway_pid = $1 AND deleted_at IS NULL
                 AND ($2::text[] IS NULL OR status = ANY($2))
               LIMIT $3"#,
            pathway_pid,
            lens.as_deref(),
            limit.map(to_i64),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// How many live instances a template has under a status lens —
    /// unbounded, unlike [`Self::cohort`] with a limit.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_cohort(
        db: &PgPool,
        pathway_pid: Uuid,
        status: Option<&str>,
    ) -> ModelResult<u64> {
        let lens = status_lens(status);
        let n = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!"
               FROM pathway_instances
               WHERE pathway_pid = $1 AND deleted_at IS NULL
                 AND ($2::text[] IS NULL OR status = ANY($2))"#,
            pathway_pid,
            lens.as_deref(),
        )
        .fetch_one(db)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// The live instances, of one template when `pathway_pid` is given,
    /// in no particular order, capped at `limit` — the flow analysis.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn live(
        db: &PgPool,
        pathway_pid: Option<Uuid>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                      next_review_on, closed_on, closure_reason, outcome,
                      clock_start_at AS "clock_start_at: _",
                      clock_stop_at AS "clock_stop_at: _",
                      deleted_at AS "deleted_at: _"
               FROM pathway_instances
               WHERE deleted_at IS NULL
                 AND ($1::uuid IS NULL OR pathway_pid = $1)
               LIMIT $2"#,
            pathway_pid,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Move a non-terminal status: writes `status` only.
    ///
    /// # Errors
    ///
    /// When the update fails, including `RowNotFound` when the row is gone.
    pub async fn set_status(self, db: &PgPool, status: &str) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE pathway_instances SET status = $2 WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                         next_review_on, closed_on, closure_reason, outcome,
                         clock_start_at AS "clock_start_at: _",
                         clock_stop_at AS "clock_stop_at: _",
                         deleted_at AS "deleted_at: _""#,
            self.id,
            status,
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// Close the instance: the terminal `status`, `closed_on`, the clock
    /// stop, the closure reason and outcome, and no further review.
    ///
    /// # Errors
    ///
    /// When the update fails, including `RowNotFound` when the row is gone.
    pub async fn close(
        self,
        db: &PgPool,
        status: &str,
        closed_on: NaiveDate,
        clock_stop_at: DateTime<FixedOffset>,
        closure_reason: Option<&str>,
        outcome: Option<&str>,
    ) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE pathway_instances
               SET status = $2, closed_on = $3, clock_stop_at = $4,
                   closure_reason = $5, outcome = $6, next_review_on = NULL
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                         next_review_on, closed_on, closure_reason, outcome,
                         clock_start_at AS "clock_start_at: _",
                         clock_stop_at AS "clock_stop_at: _",
                         deleted_at AS "deleted_at: _""#,
            self.id,
            status,
            closed_on,
            clock_stop_at,
            closure_reason,
            outcome,
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// Reschedule the next review.
    ///
    /// # Errors
    ///
    /// When the update fails, including `RowNotFound` when the row is gone.
    pub async fn set_next_review(
        self,
        db: &PgPool,
        next_review_on: NaiveDate,
    ) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE pathway_instances SET next_review_on = $2 WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                         next_review_on, closed_on, closure_reason, outcome,
                         clock_start_at AS "clock_start_at: _",
                         clock_stop_at AS "clock_stop_at: _",
                         deleted_at AS "deleted_at: _""#,
            self.id,
            next_review_on,
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// Change the urgency.
    ///
    /// # Errors
    ///
    /// When the update fails, including `RowNotFound` when the row is gone.
    pub async fn set_urgency(self, db: &PgPool, urgency: &str) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE pathway_instances SET urgency = $2 WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                         next_review_on, closed_on, closure_reason, outcome,
                         clock_start_at AS "clock_start_at: _",
                         clock_stop_at AS "clock_stop_at: _",
                         deleted_at AS "deleted_at: _""#,
            self.id,
            urgency,
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// Set the time-based-analysis clock start.
    ///
    /// # Errors
    ///
    /// When the update fails, including `RowNotFound` when the row is gone.
    pub async fn set_clock_start(
        self,
        db: &PgPool,
        at: DateTime<FixedOffset>,
    ) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE pathway_instances SET clock_start_at = $2 WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                         next_review_on, closed_on, closure_reason, outcome,
                         clock_start_at AS "clock_start_at: _",
                         clock_stop_at AS "clock_stop_at: _",
                         deleted_at AS "deleted_at: _""#,
            self.id,
            at,
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }

    /// Set the time-based-analysis clock stop.
    ///
    /// # Errors
    ///
    /// When the update fails, including `RowNotFound` when the row is gone.
    pub async fn set_clock_stop(self, db: &PgPool, at: DateTime<FixedOffset>) -> ModelResult<Self> {
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE pathway_instances SET clock_stop_at = $2 WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, pathway_pid, subject_ref, status, urgency, enrolled_on,
                         next_review_on, closed_on, closure_reason, outcome,
                         clock_start_at AS "clock_start_at: _",
                         clock_stop_at AS "clock_stop_at: _",
                         deleted_at AS "deleted_at: _""#,
            self.id,
            at,
        )
        .fetch_optional(db)
        .await?;
        gone(row)
    }
}
