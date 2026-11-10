//! Persistence for the duplicate-**review queue** (BLK-5).
//!
//! Landed as part of the bulk import/export rollout to route a
//! **keyless** import row's likely duplicate to an operator
//! (`agents/share/bulk-import-export.md` §6) rather than silently
//! withholding it or blindly creating an unreviewed duplicate. Mirrors
//! the raw-SQL `review_queue` module the person/worker/place/thing/
//! organization registries share, adapted to this crate's axa error
//! type and `chrono` timestamps — case has no pre-existing batch
//! deduplication scan, so `provenance` is part of the table from the
//! start rather than a follow-up migration (see the migration's module
//! docs).
//!
//! Pairs are stored in normalized order (`record_id_a < record_id_b`)
//! under a UNIQUE constraint, so a re-scan (or a second keyless import
//! hitting the same pair) **upserts** the same row: scores refresh, but
//! a decided row keeps its decision (`status` is never touched on
//! conflict). Statuses are stored as the family's lowercase wire tokens
//! (`pending` / `confirmed` / `rejected` / `automerged`).

use axa::prelude::*;
use axa::sqlx::Db;
use chrono::{DateTime, Utc};
use uuid::Uuid;

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
    pub created_at: DateTime<Utc>,
    /// When the decision was recorded, if decided.
    pub reviewed_at: Option<DateTime<Utc>>,
    /// The matcher's `Evidence` for the pair, as JSON
    /// (`agents/share/match-evidence.md`), refreshed on every re-scan.
    /// `None` for rows queued before the column existed.
    pub evidence: Option<serde_json::Value>,
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
    /// Initial lowercase status token: `pending`, or `automerged` only
    /// for a pair whose evidence is `proven` (see the controller's gate).
    pub status: String,
    /// How this pair was first surfaced (`operator` / `import` /
    /// `matcher_suggested`). Only applied on insert; excluded from the
    /// re-scan `DO UPDATE SET` so a re-scan never overwrites a pair's
    /// original provenance.
    pub provenance: String,
    /// The matcher's `Evidence` for the pair, as JSON. Written on insert
    /// and refreshed on a re-scan upsert, like the score columns.
    pub evidence: Option<serde_json::Value>,
}

/// Outcome of a decision attempt on one review item.
#[derive(Debug, Clone)]
pub enum DecideOutcome {
    /// The row moved from `pending` to the requested status. Boxed
    /// (BLK-5: the `provenance` field pushed `ReviewQueueRow` past
    /// clippy's `large_enum_variant` threshold relative to the other
    /// variants) so `DecideOutcome` itself stays small.
    Decided(Box<ReviewQueueRow>),
    /// No row with that id exists.
    NotFound,
    /// The row exists but is not `pending`; carries its current status.
    AlreadyDecided(String),
}

/// Insert-or-refresh a scan's candidate pairs and return the **stored**
/// rows (stable ids; a previously decided pair returns its decided
/// status untouched, with only the score columns refreshed).
///
/// # Errors
///
/// Returns [`Error::Sqlx`] if an insert fails.
pub async fn upsert(db: impl Into<Db<'_>>, items: &[NewReviewItem]) -> Result<Vec<ReviewQueueRow>> {
    let mut conn = db.into().acquire().await?;
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        // Normalize the pair order so (a, b) and (b, a) share one row.
        let (a, b) = if item.record_id_a <= item.record_id_b {
            (item.record_id_a, item.record_id_b)
        } else {
            (item.record_id_b, item.record_id_a)
        };
        let row = sqlx::query_as!(
            ReviewQueueRow,
            "INSERT INTO review_queue
                 (id, record_id_a, record_id_b, match_score, match_quality,
                  detection_method, score_breakdown, status, provenance, evidence)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (record_id_a, record_id_b) DO UPDATE SET
                 match_score = EXCLUDED.match_score,
                 match_quality = EXCLUDED.match_quality,
                 detection_method = EXCLUDED.detection_method,
                 score_breakdown = EXCLUDED.score_breakdown,
                 evidence = EXCLUDED.evidence
             RETURNING id, record_id_a, record_id_b, match_score, match_quality,
                       detection_method, score_breakdown, status, provenance,
                       reviewed_by, created_at, reviewed_at, evidence",
            Uuid::new_v4(),
            a,
            b,
            item.match_score,
            item.match_quality,
            item.detection_method,
            item.score_breakdown,
            item.status,
            item.provenance,
            item.evidence,
        )
        .fetch_one(&mut *conn)
        .await?;
        rows.push(row);
    }
    Ok(rows)
}

/// One page of the review queue, newest first, optionally filtered to
/// one status token: up to `limit` rows after skipping `offset`. The order
/// ends in the unique `id`, so a page boundary is stable. The controller
/// takes `offset` and `limit` from a cursor page it signed, so neither is
/// bounded here.
///
/// # Errors
///
/// Returns [`Error::Sqlx`] if the query fails.
pub async fn list(
    db: impl Into<Db<'_>>,
    status: Option<&str>,
    offset: u64,
    limit: u64,
) -> Result<Vec<ReviewQueueRow>> {
    let limit = i64::try_from(limit).unwrap_or(i64::MAX);
    let offset = i64::try_from(offset).unwrap_or(i64::MAX);
    let mut conn = db.into().acquire().await?;
    // One static statement: an absent filter is a NULL `$1`, which
    // matches every row.
    let rows = sqlx::query_as!(
        ReviewQueueRow,
        "SELECT id, record_id_a, record_id_b, match_score, match_quality,
                detection_method, score_breakdown, status, provenance,
                reviewed_by, created_at, reviewed_at, evidence
         FROM review_queue
         WHERE ($1::text IS NULL OR status = $1)
         ORDER BY created_at DESC, id
         LIMIT $2 OFFSET $3",
        status,
        limit,
        offset,
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok(rows)
}

/// Record an operator decision on one `pending` review item.
///
/// The transition guard lives in the SQL (`WHERE status = 'pending'`),
/// so concurrent decisions cannot double-apply: exactly one caller
/// wins, the rest observe [`DecideOutcome::AlreadyDecided`].
///
/// # Errors
///
/// Returns [`Error::Sqlx`] if a query fails.
pub async fn decide(
    db: impl Into<Db<'_>>,
    id: Uuid,
    status: &str,
    reviewed_by: Option<&str>,
) -> Result<DecideOutcome> {
    let mut conn = db.into().acquire().await?;
    let decided = sqlx::query_as!(
        ReviewQueueRow,
        "UPDATE review_queue
         SET status = $2, reviewed_by = $3, reviewed_at = now()
         WHERE id = $1 AND status = 'pending'
         RETURNING id, record_id_a, record_id_b, match_score, match_quality,
                   detection_method, score_breakdown, status, provenance,
                   reviewed_by, created_at, reviewed_at, evidence",
        id,
        status,
        reviewed_by,
    )
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(row) = decided {
        return Ok(DecideOutcome::Decided(Box::new(row)));
    }
    let current = sqlx::query_scalar!("SELECT status FROM review_queue WHERE id = $1", id)
        .fetch_optional(&mut *conn)
        .await?;
    Ok(current.map_or(DecideOutcome::NotFound, DecideOutcome::AlreadyDecided))
}
