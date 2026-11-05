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

use sea_orm::{ConnectionTrait, Statement, Value};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::Result;

/// The full column list every query returns, so `RETURNING` and
/// `SELECT` stay in lockstep with [`ReviewQueueRow`].
const COLS: &str = "id, record_id_a, record_id_b, match_score, match_quality, \
     detection_method, score_breakdown, status, reviewed_by, created_at, reviewed_at";

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
}

/// Outcome of a decision attempt on one review item.
#[derive(Debug, Clone)]
pub enum DecideOutcome {
    /// The row moved from `pending` to the requested status.
    Decided(ReviewQueueRow),
    /// No row with that id exists.
    NotFound,
    /// The row exists but is not `pending`; carries its current status.
    AlreadyDecided(String),
}

/// Map one query result onto a [`ReviewQueueRow`].
fn row_from(qr: &sea_orm::QueryResult) -> Result<ReviewQueueRow> {
    Ok(ReviewQueueRow {
        id: qr.try_get("", "id")?,
        record_id_a: qr.try_get("", "record_id_a")?,
        record_id_b: qr.try_get("", "record_id_b")?,
        match_score: qr.try_get("", "match_score")?,
        match_quality: qr.try_get("", "match_quality")?,
        detection_method: qr.try_get("", "detection_method")?,
        score_breakdown: qr.try_get("", "score_breakdown")?,
        status: qr.try_get("", "status")?,
        reviewed_by: qr.try_get("", "reviewed_by")?,
        created_at: qr.try_get("", "created_at")?,
        reviewed_at: qr.try_get("", "reviewed_at")?,
    })
}

/// Insert-or-refresh a scan's candidate pairs and return the **stored**
/// rows (stable ids; a previously decided pair returns its decided
/// status untouched, with only the score columns refreshed).
///
/// # Errors
///
/// Returns the crate database error if an insert fails.
pub async fn upsert<C: ConnectionTrait>(
    conn: &C,
    items: &[NewReviewItem],
) -> Result<Vec<ReviewQueueRow>> {
    let sql = format!(
        "INSERT INTO review_queue \
         (id, record_id_a, record_id_b, match_score, match_quality, \
          detection_method, score_breakdown, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (record_id_a, record_id_b) DO UPDATE SET \
             match_score = EXCLUDED.match_score, \
             match_quality = EXCLUDED.match_quality, \
             detection_method = EXCLUDED.detection_method, \
             score_breakdown = EXCLUDED.score_breakdown \
         RETURNING {COLS}"
    );
    let mut rows = Vec::with_capacity(items.len());
    for item in items {
        // Normalize the pair order so (a, b) and (b, a) share one row.
        let (a, b) = if item.record_id_a <= item.record_id_b {
            (item.record_id_a, item.record_id_b)
        } else {
            (item.record_id_b, item.record_id_a)
        };
        let stmt = Statement::from_sql_and_values(
            conn.get_database_backend(),
            &sql,
            [
                Uuid::new_v4().into(),
                a.into(),
                b.into(),
                item.match_score.into(),
                item.match_quality.clone().into(),
                item.detection_method.clone().into(),
                Value::Json(item.score_breakdown.clone().map(Box::new)),
                item.status.clone().into(),
            ],
        );
        if let Some(qr) = conn.query_one_raw(stmt).await? {
            rows.push(row_from(&qr)?);
        }
    }
    Ok(rows)
}

/// List stored review items, newest first, optionally filtered by
/// status token. `limit` is capped at 500.
///
/// # Errors
///
/// Returns the crate database error if the query fails.
pub async fn list<C: ConnectionTrait>(
    conn: &C,
    status: Option<&str>,
    limit: u64,
) -> Result<Vec<ReviewQueueRow>> {
    let limit = i64::try_from(limit.min(500)).unwrap_or(500);
    let backend = conn.get_database_backend();
    let stmt = if let Some(status) = status {
        Statement::from_sql_and_values(
            backend,
            format!(
                "SELECT {COLS} FROM review_queue WHERE status = $1 \
                 ORDER BY created_at DESC, id LIMIT $2"
            ),
            [status.to_string().into(), limit.into()],
        )
    } else {
        Statement::from_sql_and_values(
            backend,
            format!("SELECT {COLS} FROM review_queue ORDER BY created_at DESC, id LIMIT $1"),
            [limit.into()],
        )
    };
    let mut rows = Vec::new();
    for qr in conn.query_all_raw(stmt).await? {
        rows.push(row_from(&qr)?);
    }
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
pub async fn decide<C: ConnectionTrait>(
    conn: &C,
    id: Uuid,
    status: &str,
    reviewed_by: Option<&str>,
) -> Result<DecideOutcome> {
    let backend = conn.get_database_backend();
    let update = Statement::from_sql_and_values(
        backend,
        format!(
            "UPDATE review_queue \
             SET status = $2, reviewed_by = $3, reviewed_at = now() \
             WHERE id = $1 AND status = 'pending' \
             RETURNING {COLS}"
        ),
        [
            id.into(),
            status.to_string().into(),
            reviewed_by.map(ToString::to_string).into(),
        ],
    );
    if let Some(qr) = conn.query_one_raw(update).await? {
        return Ok(DecideOutcome::Decided(row_from(&qr)?));
    }
    let probe = Statement::from_sql_and_values(
        backend,
        "SELECT status FROM review_queue WHERE id = $1",
        [id.into()],
    );
    match conn.query_one_raw(probe).await? {
        Some(qr) => Ok(DecideOutcome::AlreadyDecided(qr.try_get("", "status")?)),
        None => Ok(DecideOutcome::NotFound),
    }
}
