//! `worker_assessments` persistence — the store behind the workforce
//! assessment endpoints (`api::rest::assessments`).
//!
//! Pure persistence over [`crate::db::models::worker_assessments`]:
//! [`insert`] records a new administration, [`list_for_worker`] reads
//! one worker's live assessments (newest first), [`find`] fetches one
//! **scoped to its worker** (so a caller cannot read or mutate another
//! worker's record through a guessed id), [`update`] replaces the
//! mutable columns, and [`soft_delete`] withdraws a record while
//! keeping the audit trail intact.
//!
//! The row ↔ domain conversion lives here too ([`to_domain`],
//! [`from_domain`]): the domain [`Assessment`] uses `chrono` dates and
//! typed enums, the row stores `time::Date` and lowercase wire tokens
//! with the per-scale results as JSONB. An unparsable token or
//! malformed `results` payload is a data-integrity error, not a panic —
//! [`to_domain`] returns [`crate::Error`] so a bad row surfaces as a
//! `500` with a message rather than taking the process down (security
//! invariant 2: never panic on stored/untrusted input).

use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder,
};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::db::convert::{date_to_time, offset_to_ts, time_to_date, ts_to_offset};
use crate::db::models::worker_assessments as row;
use crate::models::assessment::{
    Assessment, AssessmentCategory, AssessmentResult, AssessmentStatus,
};

/// Convert a stored row into the domain [`Assessment`].
///
/// # Errors
///
/// Returns [`crate::Error::Internal`] when the row carries a category
/// or status token outside the domain vocabulary, or a `results`
/// payload that is not a list of assessment results — i.e. when the
/// stored data has drifted from the model.
pub fn to_domain(m: &row::Model) -> Result<Assessment> {
    let category = AssessmentCategory::from_token(&m.category).ok_or_else(|| {
        crate::Error::Internal(format!(
            "stored assessment {} has unknown category {:?}",
            m.id, m.category
        ))
    })?;
    let status = AssessmentStatus::from_token(&m.status).ok_or_else(|| {
        crate::Error::Internal(format!(
            "stored assessment {} has unknown status {:?}",
            m.id, m.status
        ))
    })?;
    let results: Vec<AssessmentResult> =
        serde_json::from_value(m.results.clone()).map_err(|e| {
            crate::Error::Internal(format!(
                "stored assessment {} has malformed results: {e}",
                m.id
            ))
        })?;
    Ok(Assessment {
        id: m.id,
        worker_id: m.worker_id,
        category,
        instrument: m.instrument.clone(),
        provider: m.provider.clone(),
        status,
        administered_on: m.administered_on.map(time_to_date),
        expires_on: m.expires_on.map(time_to_date),
        administered_by: m.administered_by.clone(),
        notes: m.notes.clone(),
        results,
        created_at: offset_to_ts(m.created_at),
        updated_at: offset_to_ts(m.updated_at),
    })
}

/// Build the insertable active model for a domain [`Assessment`].
///
/// # Errors
///
/// Returns [`crate::Error::Internal`] when the results cannot be
/// serialized to JSON (unreachable for the domain type, mapped rather
/// than unwrapped).
fn from_domain(a: &Assessment) -> Result<row::ActiveModel> {
    let results = serde_json::to_value(&a.results)
        .map_err(|e| crate::Error::Internal(format!("cannot serialize assessment results: {e}")))?;
    Ok(row::ActiveModel {
        id: ActiveValue::set(a.id),
        worker_id: ActiveValue::set(a.worker_id),
        category: ActiveValue::set(a.category.as_str().to_string()),
        instrument: ActiveValue::set(a.instrument.clone()),
        provider: ActiveValue::set(a.provider.clone()),
        status: ActiveValue::set(a.status.as_str().to_string()),
        administered_on: ActiveValue::set(a.administered_on.map(date_to_time)),
        expires_on: ActiveValue::set(a.expires_on.map(date_to_time)),
        administered_by: ActiveValue::set(a.administered_by.clone()),
        notes: ActiveValue::set(a.notes.clone()),
        results: ActiveValue::set(results),
        created_at: ActiveValue::set(ts_to_offset(a.created_at)),
        updated_at: ActiveValue::set(ts_to_offset(a.updated_at)),
        deleted_at: ActiveValue::set(None),
    })
}

/// Record a new assessment administration.
///
/// Generic over [`ConnectionTrait`] so a caller can pass either the
/// pooled connection or its own transaction.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the insert fails, or
/// [`crate::Error::Internal`] when the results cannot be serialized.
pub async fn insert<C: ConnectionTrait>(db: &C, assessment: &Assessment) -> Result<row::Model> {
    let stored = from_domain(assessment)?.insert(db).await?;
    Ok(stored)
}

/// One worker's **live** assessments, most recently administered first
/// (rows with no administration date sort last, then by creation).
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn list_for_worker<C: ConnectionTrait>(
    db: &C,
    worker_id: Uuid,
) -> Result<Vec<row::Model>> {
    let rows = row::Entity::find()
        .filter(row::Column::WorkerId.eq(worker_id))
        .filter(row::Column::DeletedAt.is_null())
        .order_by_desc(row::Column::AdministeredOn)
        .order_by_desc(row::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(rows)
}

/// Find one live assessment by id **scoped to its worker**, so an id
/// from another worker's record is a miss rather than a leak. Returns
/// `None` when unknown, withdrawn, or owned by a different worker.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn find<C: ConnectionTrait>(
    db: &C,
    worker_id: Uuid,
    id: Uuid,
) -> Result<Option<row::Model>> {
    let found = row::Entity::find()
        .filter(row::Column::Id.eq(id))
        .filter(row::Column::WorkerId.eq(worker_id))
        .filter(row::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    Ok(found)
}

/// The mutable columns of an assessment update. `None` leaves the
/// stored value untouched; the option-valued fields
/// ([`provider`](Self::provider), [`notes`](Self::notes), the dates)
/// are `Option<Option<_>>` so a caller can distinguish "leave alone"
/// from "clear it".
#[derive(Debug, Clone, Default)]
pub struct AssessmentUpdate {
    /// New lifecycle status (already checked against the transition
    /// machine by the handler).
    pub status: Option<AssessmentStatus>,
    /// New instrument name.
    pub instrument: Option<String>,
    /// Set or clear the provider.
    pub provider: Option<Option<String>>,
    /// Set or clear the administration date.
    pub administered_on: Option<Option<chrono::NaiveDate>>,
    /// Set or clear the expiry date.
    pub expires_on: Option<Option<chrono::NaiveDate>>,
    /// Set or clear the administering identity.
    pub administered_by: Option<Option<String>>,
    /// Set or clear the operator notes.
    pub notes: Option<Option<String>>,
    /// Replace the per-scale results wholesale.
    pub results: Option<Vec<AssessmentResult>>,
}

impl AssessmentUpdate {
    /// Whether this update would change nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.instrument.is_none()
            && self.provider.is_none()
            && self.administered_on.is_none()
            && self.expires_on.is_none()
            && self.administered_by.is_none()
            && self.notes.is_none()
            && self.results.is_none()
    }
}

/// Apply an update to a stored assessment, stamping `updated_at`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the update fails, or
/// [`crate::Error::Internal`] when the new results cannot be
/// serialized.
pub async fn update<C: ConnectionTrait>(
    db: &C,
    stored: row::Model,
    change: &AssessmentUpdate,
) -> Result<row::Model> {
    let mut active: row::ActiveModel = stored.into();
    if let Some(status) = change.status {
        active.status = ActiveValue::set(status.as_str().to_string());
    }
    if let Some(instrument) = &change.instrument {
        active.instrument = ActiveValue::set(instrument.clone());
    }
    if let Some(provider) = &change.provider {
        active.provider = ActiveValue::set(provider.clone());
    }
    if let Some(administered_on) = change.administered_on {
        active.administered_on = ActiveValue::set(administered_on.map(date_to_time));
    }
    if let Some(expires_on) = change.expires_on {
        active.expires_on = ActiveValue::set(expires_on.map(date_to_time));
    }
    if let Some(administered_by) = &change.administered_by {
        active.administered_by = ActiveValue::set(administered_by.clone());
    }
    if let Some(notes) = &change.notes {
        active.notes = ActiveValue::set(notes.clone());
    }
    if let Some(results) = &change.results {
        let json = serde_json::to_value(results).map_err(|e| {
            crate::Error::Internal(format!("cannot serialize assessment results: {e}"))
        })?;
        active.results = ActiveValue::set(json);
    }
    active.updated_at = ActiveValue::set(OffsetDateTime::now_utc());
    let updated = active.update(db).await?;
    Ok(updated)
}

/// Soft-delete (withdraw) an assessment: stamp `deleted_at = now()`.
/// The row survives for the audit trail.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the update fails.
pub async fn soft_delete<C: ConnectionTrait>(db: &C, stored: row::Model) -> Result<row::Model> {
    let mut active: row::ActiveModel = stored.into();
    active.deleted_at = ActiveValue::set(Some(OffsetDateTime::now_utc()));
    active.updated_at = ActiveValue::set(OffsetDateTime::now_utc());
    let updated = active.update(db).await?;
    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::assessment::{AssessmentScale, ScoreBand};

    /// Build a stored row equivalent to `assessment`, as the DB would
    /// return it after an insert.
    fn stored_like(assessment: &Assessment) -> row::Model {
        row::Model {
            id: assessment.id,
            worker_id: assessment.worker_id,
            category: assessment.category.as_str().to_string(),
            instrument: assessment.instrument.clone(),
            provider: assessment.provider.clone(),
            status: assessment.status.as_str().to_string(),
            administered_on: assessment.administered_on.map(date_to_time),
            expires_on: assessment.expires_on.map(date_to_time),
            administered_by: assessment.administered_by.clone(),
            notes: assessment.notes.clone(),
            results: serde_json::to_value(&assessment.results).expect("results serialize"),
            created_at: ts_to_offset(assessment.created_at),
            updated_at: ts_to_offset(assessment.updated_at),
            deleted_at: None,
        }
    }

    /// A domain assessment survives the row round-trip: tokens, dates,
    /// and the JSONB results all come back intact.
    #[test]
    fn domain_row_round_trip() {
        let mut a = Assessment::new(
            Uuid::new_v4(),
            AssessmentCategory::Psychometric,
            "Hogan HPI",
        );
        a.provider = Some("Hogan".to_string());
        a.status = AssessmentStatus::Completed;
        a.administered_on = chrono::NaiveDate::from_ymd_opt(2026, 3, 1);
        a.expires_on = chrono::NaiveDate::from_ymd_opt(2028, 3, 1);
        a.administered_by = Some("hr-ops".to_string());
        a.notes = Some("remote proctored".to_string());
        a.results.push(AssessmentResult::percentile(
            AssessmentScale::EmotionalIntelligence,
            72.0,
        ));

        let back = to_domain(&stored_like(&a)).expect("row converts back");

        assert_eq!(back.id, a.id);
        assert_eq!(back.worker_id, a.worker_id);
        assert_eq!(back.category, AssessmentCategory::Psychometric);
        assert_eq!(back.status, AssessmentStatus::Completed);
        assert_eq!(back.instrument, "Hogan HPI");
        assert_eq!(back.provider.as_deref(), Some("Hogan"));
        assert_eq!(back.administered_on, a.administered_on);
        assert_eq!(back.expires_on, a.expires_on);
        assert_eq!(back.administered_by.as_deref(), Some("hr-ops"));
        assert_eq!(back.notes.as_deref(), Some("remote proctored"));
        assert_eq!(back.results.len(), 1);
        assert_eq!(
            back.results[0].scale,
            AssessmentScale::EmotionalIntelligence
        );
        assert_eq!(back.results[0].band, Some(ScoreBand::AboveAverage));
    }

    /// A row whose stored tokens or results have drifted out of the
    /// domain vocabulary is an error, never a panic (invariant 2).
    #[test]
    fn drifted_rows_error_rather_than_panic() {
        let a = Assessment::new(Uuid::new_v4(), AssessmentCategory::Aptitude, "SHL Verify");

        let mut bad_category = stored_like(&a);
        bad_category.category = "astrology".to_string();
        assert!(to_domain(&bad_category).is_err(), "unknown category");

        let mut bad_status = stored_like(&a);
        bad_status.status = "vibing".to_string();
        assert!(to_domain(&bad_status).is_err(), "unknown status");

        let mut bad_results = stored_like(&a);
        bad_results.results = serde_json::json!({ "not": "a list" });
        assert!(to_domain(&bad_results).is_err(), "malformed results");

        let mut bad_scale = stored_like(&a);
        bad_scale.results = serde_json::json!([{ "scale": "telepathy" }]);
        assert!(to_domain(&bad_scale).is_err(), "unknown scale token");
    }

    /// An empty update is recognised as a no-op, so the handler can
    /// reject it with a `422` rather than bumping `updated_at`.
    #[test]
    fn empty_update_is_detected() {
        assert!(AssessmentUpdate::default().is_empty());
        let change = AssessmentUpdate {
            status: Some(AssessmentStatus::Completed),
            ..Default::default()
        };
        assert!(!change.is_empty());
        // Clearing a field is a real change, not an empty update.
        let clear = AssessmentUpdate {
            notes: Some(None),
            ..Default::default()
        };
        assert!(!clear.is_empty());
    }

    /// DB-gated round-trip against a real Postgres (set `DATABASE_URL`):
    /// insert → the worker-scoped list and find return it → an update
    /// applies the scoring and clears a field → a find scoped to a
    /// *different* worker misses → soft-delete removes it from the live
    /// set.
    #[tokio::test]
    #[ignore = "requires DATABASE_URL to a migrated Postgres"]
    async fn round_trip_insert_find_update_delete() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let db = sea_orm::Database::connect(&url).await.expect("connect");
        let worker_id = Uuid::new_v4();

        let mut assessment = Assessment::new(
            worker_id,
            AssessmentCategory::Selection,
            "Assessment centre",
        );
        assessment.notes = Some("first sitting".to_string());
        let inserted = insert(&db, &assessment).await.expect("insert");
        assert_eq!(inserted.id, assessment.id);

        // The worker-scoped list and find both see it.
        let listed = list_for_worker(&db, worker_id).await.expect("list");
        assert!(listed.iter().any(|r| r.id == assessment.id));
        let found = find(&db, worker_id, assessment.id)
            .await
            .expect("find")
            .expect("present");

        // Another worker's id cannot reach this row.
        assert!(
            find(&db, Uuid::new_v4(), assessment.id)
                .await
                .expect("cross-worker find")
                .is_none(),
            "the lookup is worker-scoped"
        );

        // Score it and clear the notes in one update.
        let change = AssessmentUpdate {
            status: Some(AssessmentStatus::Completed),
            administered_on: Some(chrono::NaiveDate::from_ymd_opt(2026, 7, 1)),
            notes: Some(None),
            results: Some(vec![AssessmentResult::percentile(
                AssessmentScale::JobSimulation,
                88.0,
            )]),
            ..Default::default()
        };
        let updated = update(&db, found, &change).await.expect("update");
        let domain = to_domain(&updated).expect("convert");
        assert_eq!(domain.status, AssessmentStatus::Completed);
        assert!(domain.notes.is_none(), "an explicit null cleared the notes");
        assert_eq!(domain.results.len(), 1);
        assert_eq!(domain.results[0].band, Some(ScoreBand::AboveAverage));

        // Withdraw it: gone from the live set, row retained.
        soft_delete(&db, updated).await.expect("soft delete");
        let after = list_for_worker(&db, worker_id).await.expect("list again");
        assert!(
            !after.iter().any(|r| r.id == assessment.id),
            "a withdrawn assessment leaves the live set"
        );
        assert!(
            find(&db, worker_id, assessment.id)
                .await
                .expect("find after delete")
                .is_none()
        );
    }
}
