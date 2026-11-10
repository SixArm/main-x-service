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

use axa::sqlx::Db;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;
use crate::db::convert::{date_to_time, offset_to_ts, time_to_date, ts_to_offset};
use crate::db::models::{AssessmentRaw, worker_assessments as row};
use crate::db::not_updated;
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

/// A domain [`Assessment`] as the row that will be stored.
///
/// Returns a `Model`, not an `ActiveModel`, so the caller can hash exactly
/// the value it is about to write. `content_hash` is left `None` here and
/// filled in by [`write_hashed`], which is the single place a digest is
/// ever computed.
///
/// # Errors
///
/// Returns [`crate::Error::Internal`] when the results cannot be
/// serialized.
fn model_from_domain(a: &Assessment) -> Result<row::Model> {
    let results = serde_json::to_value(&a.results)
        .map_err(|e| crate::Error::Internal(format!("cannot serialize assessment results: {e}")))?;
    Ok(row::Model {
        id: a.id,
        worker_id: a.worker_id,
        category: a.category.as_str().to_string(),
        instrument: a.instrument.clone(),
        provider: a.provider.clone(),
        status: a.status.as_str().to_string(),
        administered_on: a.administered_on.map(date_to_time),
        expires_on: a.expires_on.map(date_to_time),
        administered_by: a.administered_by.clone(),
        notes: a.notes.clone(),
        results,
        created_at: ts_to_offset(a.created_at),
        updated_at: ts_to_offset(a.updated_at),
        deleted_at: None,
        content_hash: None,
        content_hash_sha3: None,
        content_mac: None,
    })
}

/// Record a new assessment administration.
///
/// Takes the pool or a caller's own connection, such as an open
/// transaction.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the insert fails, or
/// [`crate::Error::Internal`] when the results cannot be serialized.
pub async fn insert(db: impl Into<Db<'_>>, assessment: &Assessment) -> Result<row::Model> {
    write_hashed(db, model_from_domain(assessment)?, true).await
}

/// Write a row, hashing it as part of the same statement.
///
/// The digest is taken over `row` and then stored **on** `row`, so the
/// hash input and the written value are the same object — there is no
/// parallel hash-input structure that could drift out of step with the
/// columns actually persisted. Every column is written, the update
/// included, and the row comes back through `RETURNING`.
///
/// This is one statement, matching how the audit chain hashes its rows
/// before insert. The earlier implementation wrote the row and then
/// stamped the hash in a second `UPDATE` to guarantee the digest covered
/// the row *as stored*; that guarantee now comes from the round-trip test
/// (`stored_row_hash_survives_a_jsonb_round_trip`) instead, which is what
/// the audit chain already relies on for the same reason. The remaining
/// risk either way is a database-side normalisation of a hashed column —
/// JSONB key order or number formatting in `results` — and a test that
/// reads the row back is the only thing that can actually detect it. A
/// second `UPDATE` merely narrowed the window; it did not close it.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the write fails.
async fn write_hashed(
    db: impl Into<Db<'_>>,
    mut row: row::Model,
    insert: bool,
) -> Result<row::Model> {
    let d = crate::compliance::record_integrity::assessment_digests(&row);
    row.content_hash = Some(d.sha256);
    row.content_hash_sha3 = Some(d.sha3);
    row.content_mac = d.mac;
    let mut conn = db.into().acquire().await?;
    let stored = if insert {
        Some(
            sqlx::query_as!(
                AssessmentRaw,
                "INSERT INTO worker_assessments
                     (id, worker_id, category, instrument, provider, status, administered_on,
                      expires_on, administered_by, notes, results, created_at, updated_at,
                      deleted_at, content_hash, content_hash_sha3, content_mac)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                         $16, $17)
                 RETURNING id, worker_id, category, instrument, provider, status,
                           administered_on, expires_on, administered_by, notes, results,
                           created_at, updated_at, deleted_at, content_hash,
                           content_hash_sha3, content_mac",
                row.id,
                row.worker_id,
                row.category,
                row.instrument,
                row.provider,
                row.status,
                row.administered_on.map(time_to_date),
                row.expires_on.map(time_to_date),
                row.administered_by,
                row.notes,
                row.results,
                offset_to_ts(row.created_at),
                offset_to_ts(row.updated_at),
                row.deleted_at.map(offset_to_ts),
                row.content_hash,
                row.content_hash_sha3,
                row.content_mac,
            )
            .fetch_one(&mut *conn)
            .await?,
        )
    } else {
        // Every column but the key, as the `SeaORM` full-row update wrote
        // them. A row that is gone is an error, as it was there.
        sqlx::query_as!(
            AssessmentRaw,
            "UPDATE worker_assessments
             SET worker_id = $2, category = $3, instrument = $4, provider = $5, status = $6,
                 administered_on = $7, expires_on = $8, administered_by = $9, notes = $10,
                 results = $11, created_at = $12, updated_at = $13, deleted_at = $14,
                 content_hash = $15, content_hash_sha3 = $16, content_mac = $17
             WHERE id = $1
             RETURNING id, worker_id, category, instrument, provider, status,
                       administered_on, expires_on, administered_by, notes, results,
                       created_at, updated_at, deleted_at, content_hash, content_hash_sha3,
                       content_mac",
            row.id,
            row.worker_id,
            row.category,
            row.instrument,
            row.provider,
            row.status,
            row.administered_on.map(time_to_date),
            row.expires_on.map(time_to_date),
            row.administered_by,
            row.notes,
            row.results,
            offset_to_ts(row.created_at),
            offset_to_ts(row.updated_at),
            row.deleted_at.map(offset_to_ts),
            row.content_hash,
            row.content_hash_sha3,
            row.content_mac,
        )
        .fetch_optional(&mut *conn)
        .await?
    };
    stored.map(Into::into).ok_or_else(not_updated)
}

/// One worker's **live** assessments, most recently administered first
/// (rows with no administration date sort last, then by creation).
///
/// `NULLS LAST` is spelled out because Postgres sorts `NULL` *first* under
/// `DESC` (WK-A1): without it an undated, scheduled assessment came before
/// every dated one.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn list_for_worker(db: impl Into<Db<'_>>, worker_id: Uuid) -> Result<Vec<row::Model>> {
    let mut conn = db.into().acquire().await?;
    let rows = sqlx::query_as!(
        AssessmentRaw,
        "SELECT id, worker_id, category, instrument, provider, status, administered_on,
                expires_on, administered_by, notes, results, created_at, updated_at,
                deleted_at, content_hash, content_hash_sha3, content_mac
         FROM worker_assessments
         WHERE worker_id = $1 AND deleted_at IS NULL
         ORDER BY administered_on DESC NULLS LAST, created_at DESC",
        worker_id,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Find one live assessment by id **scoped to its worker**, so an id
/// from another worker's record is a miss rather than a leak. Returns
/// `None` when unknown, withdrawn, or owned by a different worker.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn find(db: impl Into<Db<'_>>, worker_id: Uuid, id: Uuid) -> Result<Option<row::Model>> {
    let mut conn = db.into().acquire().await?;
    let found = sqlx::query_as!(
        AssessmentRaw,
        "SELECT id, worker_id, category, instrument, provider, status, administered_on,
                expires_on, administered_by, notes, results, created_at, updated_at,
                deleted_at, content_hash, content_hash_sha3, content_mac
         FROM worker_assessments
         WHERE id = $1 AND worker_id = $2 AND deleted_at IS NULL
         LIMIT 1",
        id,
        worker_id,
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(found.map(Into::into))
}

/// The `limit` most recently updated assessment rows, deleted or not —
/// the assessment half of `/api/records/verify`.
///
/// Newest is `updated_at` descending, as the `workers` half orders, and
/// `id` breaks ties so the window is deterministic. With no `ORDER BY` a
/// capped run checked whichever rows the heap scan reached first, so a
/// recent edit could go unchecked (WK-A2, as LG-A1 was for audit rows).
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the query fails.
pub async fn rows_for_verification(db: impl Into<Db<'_>>, limit: i64) -> Result<Vec<row::Model>> {
    let mut conn = db.into().acquire().await?;
    let rows = sqlx::query_as!(
        AssessmentRaw,
        "SELECT id, worker_id, category, instrument, provider, status, administered_on,
                expires_on, administered_by, notes, results, created_at, updated_at,
                deleted_at, content_hash, content_hash_sha3, content_mac
         FROM worker_assessments
         ORDER BY updated_at DESC, id DESC
         LIMIT $1",
        limit,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
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
pub async fn update(
    db: impl Into<Db<'_>>,
    stored: row::Model,
    change: &AssessmentUpdate,
) -> Result<row::Model> {
    write_hashed(db, apply_change(stored, change)?, false).await
}

/// Apply an update to a stored row, returning the row as it will be
/// persisted.
///
/// Pure, and returns a `Model` rather than mutating an `ActiveModel`, so
/// the same value can be both hashed and written. Doing it the other way
/// round — setting `ActiveModel` fields and separately assembling a hash
/// input — is what would let the digest describe something other than the
/// stored row.
fn apply_change(mut row: row::Model, change: &AssessmentUpdate) -> Result<row::Model> {
    if let Some(status) = change.status {
        row.status = status.as_str().to_string();
    }
    if let Some(instrument) = &change.instrument {
        row.instrument.clone_from(instrument);
    }
    if let Some(provider) = &change.provider {
        row.provider.clone_from(provider);
    }
    if let Some(administered_on) = change.administered_on {
        row.administered_on = administered_on.map(date_to_time);
    }
    if let Some(expires_on) = change.expires_on {
        row.expires_on = expires_on.map(date_to_time);
    }
    if let Some(administered_by) = &change.administered_by {
        row.administered_by.clone_from(administered_by);
    }
    if let Some(notes) = &change.notes {
        row.notes.clone_from(notes);
    }
    if let Some(results) = &change.results {
        row.results = serde_json::to_value(results).map_err(|e| {
            crate::Error::Internal(format!("cannot serialize assessment results: {e}"))
        })?;
    }
    row.updated_at = OffsetDateTime::now_utc();
    Ok(row)
}

/// Soft-delete (withdraw) an assessment: stamp `deleted_at = now()`.
/// The row survives for the audit trail.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] when the update fails.
pub async fn soft_delete(db: impl Into<Db<'_>>, mut stored: row::Model) -> Result<row::Model> {
    // A withdrawal changes `deleted_at`, which the digest binds, so the
    // row rehashes like any other write.
    let now = OffsetDateTime::now_utc();
    stored.deleted_at = Some(now);
    stored.updated_at = now;
    write_hashed(db, stored, false).await
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
            // Unhashed: `write_hashed` fills both in as it writes.
            content_hash: None,
            content_hash_sha3: None,
            content_mac: None,
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

    /// **The load-bearing pin for the single-write design.** A row's
    /// stored hash still verifies after a full Postgres round trip.
    ///
    /// The digest is computed in the same statement that writes the row,
    /// so it describes the value *sent* to Postgres. If Postgres then
    /// normalises a hashed column on the way in — JSONB key ordering or
    /// number formatting in `results` is the realistic candidate — the
    /// value read back would hash differently and every untouched
    /// assessment would report as tampered. Nothing but reading the row
    /// back can detect that.
    ///
    /// This is the same property, and the same reasoning, as the audit
    /// chain's `chain_survives_a_jsonb_round_trip`. It is what allows one
    /// write instead of write-then-stamp: the earlier two-statement form
    /// narrowed this window but never closed it, because the second
    /// statement hashed a value that had itself already round-tripped
    /// once.
    #[tokio::test]
    #[ignore = "requires DB_URL to a migrated Postgres"]
    async fn stored_row_hash_survives_a_jsonb_round_trip() {
        use crate::compliance::record_integrity::{assessment_hash, verify_assessments};

        let url = std::env::var("DB_URL").expect("DB_URL");
        let db = sqlx::PgPool::connect(&url).await.expect("connect");
        let worker_id = Uuid::new_v4();

        // Awkward numbers on purpose: a whole float, a repeating decimal,
        // and a value with more precision than the band logic needs. These
        // are what a naive JSONB normalisation would round or reformat.
        let mut assessment =
            Assessment::new(worker_id, AssessmentCategory::Psychometric, "Round Trip");
        assessment.results = vec![
            AssessmentResult::percentile(AssessmentScale::NumericalReasoning, 100.0),
            AssessmentResult::percentile(AssessmentScale::VerbalReasoning, 33.333_333_333_333_336),
            AssessmentResult::percentile(AssessmentScale::ProblemSolving, 0.5),
        ];

        let inserted = insert(&db, &assessment).await.expect("insert");
        let stored_hash = inserted
            .content_hash
            .clone()
            .expect("the insert stamped a hash");

        // Read the row back through Postgres and rehash it.
        let read_back = find(&db, worker_id, assessment.id)
            .await
            .expect("find")
            .expect("present");
        assert_eq!(
            assessment_hash(&read_back),
            stored_hash,
            "Postgres returned a value that hashes differently from the one written — \
             every untouched assessment would report as tampered"
        );
        assert!(verify_assessments(&[read_back]).verified);

        // The same must hold after an update, whose results go through a
        // second serialization.
        let found = find(&db, worker_id, assessment.id)
            .await
            .expect("find")
            .expect("present");
        let updated = update(
            &db,
            found,
            &AssessmentUpdate {
                results: Some(vec![AssessmentResult::percentile(
                    AssessmentScale::LogicalThinking,
                    66.666_666_666_666_67,
                )]),
                ..Default::default()
            },
        )
        .await
        .expect("update");
        let read_back = find(&db, worker_id, assessment.id)
            .await
            .expect("find")
            .expect("present");
        assert_eq!(
            read_back.content_hash, updated.content_hash,
            "the update's hash must match what the database stored"
        );
        assert!(verify_assessments(&[read_back]).verified);

        // And after a soft delete, which changes a bound field.
        let found = find(&db, worker_id, assessment.id)
            .await
            .expect("find")
            .expect("present");
        let deleted = soft_delete(&db, found).await.expect("soft delete");
        assert!(
            verify_assessments(&[deleted]).verified,
            "a withdrawn assessment must still verify"
        );
    }

    /// DB-gated round-trip against a real Postgres (set `DB_URL`):
    /// insert → the worker-scoped list and find return it → an update
    /// applies the scoring and clears a field → a find scoped to a
    /// *different* worker misses → soft-delete removes it from the live
    /// set.
    #[tokio::test]
    #[ignore = "requires DB_URL to a migrated Postgres"]
    async fn round_trip_insert_find_update_delete() {
        let url = std::env::var("DB_URL").expect("DB_URL");
        let db = sqlx::PgPool::connect(&url).await.expect("connect");
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
