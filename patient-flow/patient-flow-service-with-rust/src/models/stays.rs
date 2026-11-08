//! `stays` — one inpatient episode for one patient (`person_ref` URN),
//! admission → transfers → discharge, with the SAFER fields and the
//! DTOC anchor (spec `domain-model.md`).

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// One `stays` row, serialized as-is by the stay endpoints (the wire
/// shape the `SeaORM` entity had).
#[allow(missing_docs)] // the columns are documented by the migration
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub created_at: DateTime<FixedOffset>,
    pub updated_at: DateTime<FixedOffset>,
    pub id: i64,
    pub pid: Uuid,
    pub person_ref: String,
    pub display_name: String,
    pub status: String,
    pub admitted_at: DateTime<FixedOffset>,
    pub source: String,
    pub ward_pid: Option<Uuid>,
    pub bed_pid: Option<Uuid>,
    pub home_location_note: Option<String>,
    pub named_nurse_ref: Option<String>,
    pub consultant_ref: Option<String>,
    pub senior_review_at: Option<DateTime<FixedOffset>>,
    pub edd: Option<NaiveDate>,
    pub ccd: Option<String>,
    pub ccd_met: bool,
    pub discharge_pathway: Option<String>,
    pub discharge_ready_at: Option<DateTime<FixedOffset>>,
    pub discharged_at: Option<DateTime<FixedOffset>>,
    pub discharge_destination: Option<String>,
    pub alerts: serde_json::Value,
    pub deleted_at: Option<DateTime<FixedOffset>>,
}

/// The columns of a new stay. A new stay has not had a senior review,
/// has not met its discharge criteria, and is not discharge-ready,
/// discharged or deleted.
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub struct NewStay<'a> {
    pub pid: Uuid,
    pub person_ref: &'a str,
    pub display_name: &'a str,
    pub status: &'a str,
    pub admitted_at: DateTime<FixedOffset>,
    pub source: &'a str,
    pub ward_pid: Option<Uuid>,
    pub bed_pid: Option<Uuid>,
    pub home_location_note: Option<&'a str>,
    pub named_nurse_ref: Option<&'a str>,
    pub consultant_ref: Option<&'a str>,
    pub edd: Option<NaiveDate>,
    pub ccd: Option<&'a str>,
    pub alerts: serde_json::Value,
}

/// A partial update of the whiteboard-editable fields: `None` leaves the
/// column as it is, which is what the `SeaORM` era's `ActiveModel`
/// update did with an unset field.
#[allow(missing_docs)]
#[derive(Debug, Clone, Default)]
pub struct StayChanges {
    pub display_name: Option<String>,
    pub named_nurse_ref: Option<String>,
    pub consultant_ref: Option<String>,
    pub edd: Option<NaiveDate>,
    pub ccd: Option<String>,
    pub ccd_met: Option<bool>,
    pub senior_review_at: Option<DateTime<FixedOffset>>,
    pub alerts: Option<serde_json::Value>,
    pub home_location_note: Option<String>,
}

impl Model {
    /// Insert a stay, returning the row.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn insert(db: impl Into<Db<'_>>, new: &NewStay<'_>) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO stays (pid, person_ref, display_name, status, admitted_at, source,
                                  ward_pid, bed_pid, home_location_note, named_nurse_ref,
                                  consultant_ref, senior_review_at, edd, ccd, ccd_met,
                                  discharge_pathway, discharge_ready_at, discharged_at,
                                  discharge_destination, alerts, deleted_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NULL, $12, $13, false,
                       NULL, NULL, NULL, NULL, $14, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, person_ref, display_name, status,
                         admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                         home_location_note, named_nurse_ref, consultant_ref,
                         senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                         discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                         discharged_at AS "discharged_at: _", discharge_destination, alerts,
                         deleted_at AS "deleted_at: _""#,
            new.pid,
            new.person_ref,
            new.display_name,
            new.status,
            new.admitted_at,
            new.source,
            new.ward_pid,
            new.bed_pid,
            new.home_location_note,
            new.named_nurse_ref,
            new.consultant_ref,
            new.edd,
            new.ccd,
            new.alerts,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// The active (not soft-deleted) stay with this `pid`.
    ///
    /// # Errors
    ///
    /// [`ModelError::EntityNotFound`] when absent or soft-deleted.
    pub async fn find_active(db: impl Into<Db<'_>>, pid: Uuid) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, person_ref, display_name, status,
                      admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                      home_location_note, named_nurse_ref, consultant_ref,
                      senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                      discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                      discharged_at AS "discharged_at: _", discharge_destination, alerts,
                      deleted_at AS "deleted_at: _"
               FROM stays
               WHERE pid = $1 AND deleted_at IS NULL"#,
            pid,
        )
        .fetch_optional(&mut *conn)
        .await?
        .ok_or(ModelError::EntityNotFound)
    }

    /// Whether an active, undischarged stay occupies this bed — the
    /// one-occupant invariant. Run it on the transaction that holds the
    /// bed's lock.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn bed_is_occupied(db: impl Into<Db<'_>>, bed_pid: Uuid) -> ModelResult<bool> {
        let mut conn = db.into().acquire().await?;
        let occupant = sqlx::query_scalar!(
            r#"SELECT id FROM stays
               WHERE bed_pid = $1 AND discharged_at IS NULL AND deleted_at IS NULL
               LIMIT 1"#,
            bed_pid,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(occupant.is_some())
    }

    /// The active, undischarged stays on one ward (the whiteboard).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn current_in_ward(db: impl Into<Db<'_>>, ward_pid: Uuid) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, person_ref, display_name, status,
                      admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                      home_location_note, named_nurse_ref, consultant_ref,
                      senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                      discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                      discharged_at AS "discharged_at: _", discharge_destination, alerts,
                      deleted_at AS "deleted_at: _"
               FROM stays
               WHERE ward_pid = $1 AND discharged_at IS NULL AND deleted_at IS NULL"#,
            ward_pid,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Every active, undischarged stay (the at-a-glance estate walk).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn all_current(db: impl Into<Db<'_>>) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, person_ref, display_name, status,
                      admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                      home_location_note, named_nurse_ref, consultant_ref,
                      senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                      discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                      discharged_at AS "discharged_at: _", discharge_destination, alerts,
                      deleted_at AS "deleted_at: _"
               FROM stays
               WHERE discharged_at IS NULL AND deleted_at IS NULL"#,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// The latest (highest id) active stay of one patient, discharged or
    /// not (the locate read).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn latest_for_person(
        db: impl Into<Db<'_>>,
        person_ref: &str,
    ) -> ModelResult<Option<Self>> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, pid, person_ref, display_name, status,
                      admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                      home_location_note, named_nurse_ref, consultant_ref,
                      senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                      discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                      discharged_at AS "discharged_at: _", discharge_destination, alerts,
                      deleted_at AS "deleted_at: _"
               FROM stays
               WHERE person_ref = $1 AND deleted_at IS NULL
               ORDER BY id DESC
               LIMIT 1"#,
            person_ref,
        )
        .fetch_optional(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Apply a partial update of the whiteboard fields to the row with
    /// this `id`, returning it.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn update(
        db: impl Into<Db<'_>>,
        id: i64,
        changes: &StayChanges,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE stays
               SET updated_at = now(), display_name = COALESCE($2, display_name),
                   named_nurse_ref = COALESCE($3, named_nurse_ref),
                   consultant_ref = COALESCE($4, consultant_ref),
                   edd = COALESCE($5, edd),
                   ccd = COALESCE($6, ccd),
                   ccd_met = COALESCE($7, ccd_met),
                   senior_review_at = COALESCE($8, senior_review_at),
                   alerts = COALESCE($9, alerts),
                   home_location_note = COALESCE($10, home_location_note)
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, person_ref, display_name, status,
                         admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                         home_location_note, named_nurse_ref, consultant_ref,
                         senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                         discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                         discharged_at AS "discharged_at: _", discharge_destination, alerts,
                         deleted_at AS "deleted_at: _""#,
            id,
            changes.display_name,
            changes.named_nurse_ref,
            changes.consultant_ref,
            changes.edd,
            changes.ccd,
            changes.ccd_met,
            changes.senior_review_at,
            changes.alerts,
            changes.home_location_note,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Move the stay to another ward and bed (a transfer), returning it.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn set_location(
        db: impl Into<Db<'_>>,
        id: i64,
        ward_pid: Uuid,
        bed_pid: Uuid,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE stays SET updated_at = now(), ward_pid = $2, bed_pid = $3
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, person_ref, display_name, status,
                         admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                         home_location_note, named_nurse_ref, consultant_ref,
                         senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                         discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                         discharged_at AS "discharged_at: _", discharge_destination, alerts,
                         deleted_at AS "deleted_at: _""#,
            id,
            ward_pid,
            bed_pid,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Mark the stay discharge-ready on `pathway`, starting the DTOC
    /// clock at `at`, returning it.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn mark_discharge_ready(
        db: impl Into<Db<'_>>,
        id: i64,
        at: DateTime<FixedOffset>,
        pathway: &str,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE stays
               SET updated_at = now(), status = 'discharge_ready', discharge_ready_at = $2, discharge_pathway = $3
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, person_ref, display_name, status,
                         admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                         home_location_note, named_nurse_ref, consultant_ref,
                         senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                         discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                         discharged_at AS "discharged_at: _", discharge_destination, alerts,
                         deleted_at AS "deleted_at: _""#,
            id,
            at,
            pathway,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Discharge the stay to `destination` at `at`, clearing its bed,
    /// returning it.
    ///
    /// # Errors
    ///
    /// When the update fails, or no row has this `id`.
    pub async fn discharge(
        db: impl Into<Db<'_>>,
        id: i64,
        at: DateTime<FixedOffset>,
        destination: &str,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE stays
               SET updated_at = now(), status = 'discharged', discharged_at = $2, discharge_destination = $3,
                   bed_pid = NULL
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, pid, person_ref, display_name, status,
                         admitted_at AS "admitted_at: _", source, ward_pid, bed_pid,
                         home_location_note, named_nurse_ref, consultant_ref,
                         senior_review_at AS "senior_review_at: _", edd, ccd, ccd_met,
                         discharge_pathway, discharge_ready_at AS "discharge_ready_at: _",
                         discharged_at AS "discharged_at: _", discharge_destination, alerts,
                         deleted_at AS "deleted_at: _""#,
            id,
            at,
            destination,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }
}
