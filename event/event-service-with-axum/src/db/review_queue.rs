//! Persistence for the batch-deduplication **review queue** (spec §13
//! T-15): the stored counterpart of the scan report, so review decisions
//! survive restarts and re-scans.
//!
//! Pairs are stored in normalized order (`record_id_a < record_id_b`)
//! under a UNIQUE constraint, so a re-scan **upserts** the same row:
//! scores and evidence refresh, but a decided row keeps its decision
//! (`status` is never touched on conflict). Statuses are stored as the
//! family's lowercase wire tokens (`pending` / `confirmed` / `rejected` /
//! `automerged`); the handlers own the enum mapping.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::Result;

/// One stored review-queue row. `status` is the lowercase wire token.
#[derive(Debug, Clone)]
pub struct ReviewQueueRow {
    /// Stable review-item id (survives re-scans).
    pub id: Uuid,
    /// First event of the normalized pair.
    pub record_id_a: Uuid,
    /// Second event of the normalized pair.
    pub record_id_b: Uuid,
    /// Overall match score in `[0.0, 1.0]` (refreshed on re-scan).
    pub match_score: f64,
    /// Confidence band label.
    pub match_quality: String,
    /// How the pair was detected.
    pub detection_method: String,
    /// Optional per-component score breakdown.
    pub score_breakdown: Option<serde_json::Value>,
    /// The matcher's match evidence for the pair
    /// (`agents/share/match-evidence.md`), refreshed on re-scan.
    pub evidence: Option<serde_json::Value>,
    /// Lowercase status token (`pending` / `confirmed` / `rejected` /
    /// `automerged`).
    pub status: String,
    /// Reviewer identity recorded by the decision endpoint.
    pub reviewed_by: Option<String>,
    /// When the pair was first queued.
    pub created_at: DateTime<Utc>,
    /// When the decision (or automatic merge) was recorded.
    pub reviewed_at: Option<DateTime<Utc>>,
}

/// A candidate pair from a scan, to insert-or-refresh.
#[derive(Debug, Clone)]
pub struct NewReviewItem {
    /// First event of the pair (order-insensitive; normalized on write).
    pub record_id_a: Uuid,
    /// Second event of the pair.
    pub record_id_b: Uuid,
    /// Overall match score in `[0.0, 1.0]`.
    pub match_score: f64,
    /// Confidence band label.
    pub match_quality: String,
    /// How the pair was detected.
    pub detection_method: String,
    /// Optional per-component score breakdown.
    pub score_breakdown: Option<serde_json::Value>,
    /// The matcher's match evidence for the pair, serialized.
    pub evidence: Option<serde_json::Value>,
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

/// Insert-or-refresh a scan's candidate pairs and return the **stored**
/// rows, in the order given. A new pair is stored `pending`; a pair
/// already stored keeps its status (a person's decision, or an earlier
/// automatic merge) and refreshes only its score and evidence columns.
///
/// # Errors
///
/// Returns the crate database error if an insert fails.
pub async fn upsert(pool: &PgPool, items: &[NewReviewItem]) -> Result<Vec<ReviewQueueRow>> {
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
                  detection_method, score_breakdown, evidence, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending')
             ON CONFLICT (record_id_a, record_id_b) DO UPDATE SET
                 match_score = EXCLUDED.match_score,
                 match_quality = EXCLUDED.match_quality,
                 detection_method = EXCLUDED.detection_method,
                 score_breakdown = EXCLUDED.score_breakdown,
                 evidence = EXCLUDED.evidence
             RETURNING id, record_id_a, record_id_b, match_score, match_quality,
                       detection_method, score_breakdown, evidence, status, reviewed_by,
                       created_at, reviewed_at",
            Uuid::new_v4(),
            a,
            b,
            item.match_score,
            item.match_quality,
            item.detection_method,
            item.score_breakdown,
            item.evidence,
        )
        .fetch_one(pool)
        .await?;
        rows.push(row);
    }
    Ok(rows)
}

/// One page of the review queue, newest first, optionally filtered to
/// one status token: up to `limit` rows after skipping `offset`. The order
/// ends in the unique `id`, so a page boundary is stable.
///
/// `None` lists every status: the filter is the one bound parameter
/// `$1::text IS NULL OR status = $1`, so both cases are the same static
/// statement.
///
/// # Errors
///
/// Returns the crate database error if the query fails.
pub async fn list(
    pool: &PgPool,
    status: Option<&str>,
    offset: u64,
    limit: u64,
) -> Result<Vec<ReviewQueueRow>> {
    let limit = i64::try_from(limit).unwrap_or(i64::MAX);
    let offset = i64::try_from(offset).unwrap_or(i64::MAX);
    let rows = sqlx::query_as!(
        ReviewQueueRow,
        "SELECT id, record_id_a, record_id_b, match_score, match_quality,
                detection_method, score_breakdown, evidence, status, reviewed_by,
                created_at, reviewed_at
         FROM review_queue
         WHERE ($1::text IS NULL OR status = $1)
         ORDER BY created_at DESC, id
         LIMIT $2 OFFSET $3",
        status,
        limit,
        offset,
    )
    .fetch_all(pool)
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
/// Returns the crate database error if a query fails.
pub async fn decide(
    pool: &PgPool,
    id: Uuid,
    status: &str,
    reviewed_by: Option<&str>,
) -> Result<DecideOutcome> {
    let decided = sqlx::query_as!(
        ReviewQueueRow,
        "UPDATE review_queue
         SET status = $2, reviewed_by = $3, reviewed_at = now()
         WHERE id = $1 AND status = 'pending'
         RETURNING id, record_id_a, record_id_b, match_score, match_quality,
                   detection_method, score_breakdown, evidence, status, reviewed_by,
                   created_at, reviewed_at",
        id,
        status,
        reviewed_by,
    )
    .fetch_optional(pool)
    .await?;
    if let Some(row) = decided {
        return Ok(DecideOutcome::Decided(Box::new(row)));
    }
    let current = sqlx::query_scalar!("SELECT status FROM review_queue WHERE id = $1", id)
        .fetch_optional(pool)
        .await?;
    Ok(match current {
        Some(status) => DecideOutcome::AlreadyDecided(status),
        None => DecideOutcome::NotFound,
    })
}

/// Mark a `pending` row `automerged` once the scan has merged its pair.
/// Returns the updated row, or `None` when the row is no longer `pending`
/// (a person decided it while the scan ran): the merge stands, and the
/// row keeps the person's status.
///
/// # Errors
///
/// Returns the crate database error if the query fails.
pub async fn mark_automerged(pool: &PgPool, id: Uuid) -> Result<Option<ReviewQueueRow>> {
    let row = sqlx::query_as!(
        ReviewQueueRow,
        "UPDATE review_queue
         SET status = 'automerged', reviewed_at = now()
         WHERE id = $1 AND status = 'pending'
         RETURNING id, record_id_a, record_id_b, match_score, match_quality,
                   detection_method, score_breakdown, evidence, status, reviewed_by,
                   created_at, reviewed_at",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
