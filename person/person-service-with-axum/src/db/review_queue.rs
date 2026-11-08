//! Persistence for the batch-deduplication **review queue** — the
//! stored counterpart of the scan report, so review decisions survive
//! restarts and re-scans.
//!
//! Pairs are stored in normalized order (`record_id_a < record_id_b`)
//! under a UNIQUE constraint, so a re-scan **upserts** the same row:
//! scores refresh, but a decided row keeps its decision (`status` is
//! never touched on conflict). Statuses are stored as the family's
//! lowercase wire tokens (`pending` / `confirmed` / `rejected` /
//! `automerged`); the handlers own the enum mapping.

use axa::sqlx::Db;
use chrono::{DateTime, Utc};
use time::OffsetDateTime;
use uuid::Uuid;

use super::convert::ts_to_offset;
use crate::Result;

/// One stored review-queue row. `status` is the lowercase wire token.
#[derive(Debug, Clone)]
pub struct ReviewQueueRow {
    /// Stable review-item id (survives re-scans).
    pub id: Uuid,
    /// First record of the normalized pair.
    pub record_id_a: Uuid,
    /// Second record of the normalized pair.
    pub record_id_b: Uuid,
    /// Overall match score in `[0.0, 1.0]` (refreshed on re-scan).
    pub match_score: f64,
    /// Confidence band label.
    pub match_quality: String,
    /// How the pair was detected.
    pub detection_method: String,
    /// Optional per-component score breakdown.
    pub score_breakdown: Option<serde_json::Value>,
    /// Lowercase status token (`pending` / `confirmed` / `rejected` /
    /// `automerged`).
    pub status: String,
    /// How this pair was first surfaced (`operator` / `import` /
    /// `matcher_suggested` — the cross-service-linking provenance
    /// vocabulary). Never touched on a re-scan upsert, so it always
    /// reflects the pair's origin, not its most recent detection.
    pub provenance: String,
    /// Reviewer identity recorded by the decision endpoint.
    pub reviewed_by: Option<String>,
    /// When the pair was first queued.
    pub created_at: OffsetDateTime,
    /// When the decision was recorded, if decided.
    pub reviewed_at: Option<OffsetDateTime>,
}

/// A candidate pair from a scan, to insert-or-refresh.
#[derive(Debug, Clone)]
pub struct NewReviewItem {
    /// First record of the pair (order-insensitive; normalized on write).
    pub record_id_a: Uuid,
    /// Second record of the pair.
    pub record_id_b: Uuid,
    /// Overall match score in `[0.0, 1.0]`.
    pub match_score: f64,
    /// Confidence band label.
    pub match_quality: String,
    /// How the pair was detected.
    pub detection_method: String,
    /// Optional per-component score breakdown.
    pub score_breakdown: Option<serde_json::Value>,
    /// Initial lowercase status token (`pending` or `automerged`).
    pub status: String,
    /// How this pair was first surfaced (`operator` / `import` /
    /// `matcher_suggested`). Only applied on insert; excluded from the
    /// re-scan `DO UPDATE SET` so a re-scan never overwrites a pair's
    /// original provenance.
    pub provenance: String,
}

/// Outcome of a decision attempt on one review item.
#[derive(Debug, Clone)]
pub enum DecideOutcome {
    /// The row moved from `pending` to the requested status.
    Decided(Box<ReviewQueueRow>),
    /// No row with that id exists.
    NotFound,
    /// The row exists but is not `pending`; carries its current status.
    AlreadyDecided(String),
}

/// The `chrono` twin of [`ReviewQueueRow`] that `query_as!` fills. Every
/// statement returns the same column list, so `RETURNING` and `SELECT`
/// stay in lockstep with it.
struct ReviewQueueRaw {
    id: Uuid,
    record_id_a: Uuid,
    record_id_b: Uuid,
    match_score: f64,
    match_quality: String,
    detection_method: String,
    score_breakdown: Option<serde_json::Value>,
    status: String,
    provenance: String,
    reviewed_by: Option<String>,
    created_at: DateTime<Utc>,
    reviewed_at: Option<DateTime<Utc>>,
}

impl From<ReviewQueueRaw> for ReviewQueueRow {
    fn from(r: ReviewQueueRaw) -> Self {
        Self {
            id: r.id,
            record_id_a: r.record_id_a,
            record_id_b: r.record_id_b,
            match_score: r.match_score,
            match_quality: r.match_quality,
            detection_method: r.detection_method,
            score_breakdown: r.score_breakdown,
            status: r.status,
            provenance: r.provenance,
            reviewed_by: r.reviewed_by,
            created_at: ts_to_offset(r.created_at),
            reviewed_at: r.reviewed_at.map(ts_to_offset),
        }
    }
}

/// Insert-or-refresh one pair exactly as given, returning the stored row.
/// `provenance` is excluded from the `DO UPDATE SET`, so a re-scan never
/// overwrites a pair's original provenance.
async fn upsert_one(
    conn: &mut sqlx::PgConnection,
    a: Uuid,
    b: Uuid,
    item: &NewReviewItem,
) -> Result<Option<ReviewQueueRow>> {
    let row = sqlx::query_as!(
        ReviewQueueRaw,
        "INSERT INTO review_queue
             (id, record_id_a, record_id_b, match_score, match_quality,
              detection_method, score_breakdown, status, provenance)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         ON CONFLICT (record_id_a, record_id_b) DO UPDATE SET
             match_score = EXCLUDED.match_score,
             match_quality = EXCLUDED.match_quality,
             detection_method = EXCLUDED.detection_method,
             score_breakdown = EXCLUDED.score_breakdown
         RETURNING id, record_id_a, record_id_b, match_score, match_quality,
                   detection_method, score_breakdown, status, provenance, reviewed_by,
                   created_at, reviewed_at",
        Uuid::new_v4(),
        a,
        b,
        item.match_score,
        item.match_quality,
        item.detection_method,
        item.score_breakdown,
        item.status,
        item.provenance,
    )
    .fetch_optional(&mut *conn)
    .await?;
    Ok(row.map(Into::into))
}

/// Insert-or-refresh a scan's candidate pairs and return the **stored**
/// rows (stable ids; a previously decided pair returns its decided
/// status untouched, with only the score columns refreshed).
///
/// # Errors
///
/// Returns the crate database error if an insert fails.
pub async fn upsert(
    conn: impl Into<Db<'_>>,
    items: &[NewReviewItem],
) -> Result<Vec<ReviewQueueRow>> {
    let mut conn = conn.into().acquire().await?;
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        // Normalize the pair order so (a, b) and (b, a) share one row.
        let (a, b) = if item.record_id_a <= item.record_id_b {
            (item.record_id_a, item.record_id_b)
        } else {
            (item.record_id_b, item.record_id_a)
        };
        if let Some(row) = upsert_one(&mut conn, a, b, item).await? {
            rows.push(row);
        }
    }
    Ok(rows)
}

/// Insert-or-refresh a **cross-service** suggestion row (T-32,
/// `agents/share/cross-service-linking.md` §5.2, `spec/16-open-questions.md`
/// OQ-9(b)) — the review-queue counterpart of a `matcher_suggested`
/// `same_identity` link POST.
///
/// Unlike [`upsert`], this does **not** normalize `(record_id_a,
/// record_id_b)` by raw `Uuid` comparison. `upsert`'s reordering is
/// correct for its own use (within-entity dedup, where both ids are the
/// same entity type and "which column" is meaningless) but would be
/// actively wrong here: a cross-service pair's two columns are **not**
/// interchangeable — `record_id_a` must always be the person pid and
/// `record_id_b` always the worker pid, so a reader (crucially, this
/// crate's own `review_decision` promotion path, which has to resolve
/// "which id is the worker" without a `to_ref` column to consult) can
/// tell them apart without guessing. Reordering by raw `Uuid` value would
/// silently swap the two columns for roughly half of all pairs — whichever
/// side happens to sort first as a `Uuid` — breaking that convention
/// invisibly. The pair is therefore stored **exactly as given** in
/// `item`. This is safe against the same `UNIQUE (record_id_a,
/// record_id_b)` constraint `upsert` uses: a person pid and a worker pid
/// are independent random UUID spaces, so an accidental collision with an
/// unrelated within-entity dedup pair is not a realistic concern, and in
/// any case this function is the **only** writer of cross-service rows —
/// within-entity code paths only ever call `upsert`.
///
/// `provenance` is excluded from the `DO UPDATE SET`, exactly as `upsert`
/// does, so a re-scan (the periodic suggestion job re-running) never
/// overwrites a pair's original provenance.
///
/// # Errors
///
/// Returns the crate database error if the insert fails, or
/// [`crate::Error::Api`] in the (should-never-happen) case that the
/// `RETURNING` clause produces no row.
pub async fn upsert_cross_service(
    conn: impl Into<Db<'_>>,
    item: &NewReviewItem,
) -> Result<ReviewQueueRow> {
    let mut conn = conn.into().acquire().await?;
    upsert_one(&mut conn, item.record_id_a, item.record_id_b, item)
        .await?
        .ok_or_else(|| {
            crate::Error::Api("cross-service review-queue upsert returned no row".to_string())
        })
}

/// List stored review items, newest first, optionally filtered by
/// status token. `limit` is capped at 500.
///
/// # Errors
///
/// Returns the crate database error if the query fails.
pub async fn list(
    conn: impl Into<Db<'_>>,
    status: Option<&str>,
    limit: u64,
) -> Result<Vec<ReviewQueueRow>> {
    let limit = i64::try_from(limit.min(500)).unwrap_or(500);
    let mut conn = conn.into().acquire().await?;
    let rows = sqlx::query_as!(
        ReviewQueueRaw,
        "SELECT id, record_id_a, record_id_b, match_score, match_quality,
                detection_method, score_breakdown, status, provenance, reviewed_by,
                created_at, reviewed_at
         FROM review_queue
         WHERE ($1::text IS NULL OR status = $1)
         ORDER BY created_at DESC, id
         LIMIT $2",
        status,
        limit,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows.into_iter().map(Into::into).collect())
}

/// Record an operator decision on one `pending` review item.
///
/// The transition guard lives in the SQL (`WHERE status = 'pending'`),
/// so concurrent decisions cannot double-apply: exactly one caller
/// wins, the rest observe [`DecideOutcome::AlreadyDecided`].
///
/// # Errors
///
/// Returns the crate database error if a query fails.
pub async fn decide(
    conn: impl Into<Db<'_>>,
    id: Uuid,
    status: &str,
    reviewed_by: Option<&str>,
) -> Result<DecideOutcome> {
    let mut conn = conn.into().acquire().await?;
    let decided = sqlx::query_as!(
        ReviewQueueRaw,
        "UPDATE review_queue
         SET status = $2, reviewed_by = $3, reviewed_at = now()
         WHERE id = $1 AND status = 'pending'
         RETURNING id, record_id_a, record_id_b, match_score, match_quality,
                   detection_method, score_breakdown, status, provenance, reviewed_by,
                   created_at, reviewed_at",
        id,
        status,
        reviewed_by,
    )
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(row) = decided {
        return Ok(DecideOutcome::Decided(Box::new(row.into())));
    }
    let current = sqlx::query_scalar!("SELECT status FROM review_queue WHERE id = $1", id)
        .fetch_optional(&mut *conn)
        .await?;
    match current {
        Some(status) => Ok(DecideOutcome::AlreadyDecided(status)),
        None => Ok(DecideOutcome::NotFound),
    }
}
