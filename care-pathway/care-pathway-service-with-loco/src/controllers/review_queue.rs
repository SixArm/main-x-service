//! The stored duplicate-**review queue** (§13 T-10, the native bulk
//! import/export API): `GET /api/care-pathways/review-queue` +
//! `POST /api/care-pathways/review-queue/{id}/decision`.
//!
//! Every sibling entity that has a review queue
//! (person / worker / place / thing / organization,
//! `agents/share/match-search-merge.md` "Review queue"; case added its
//! own during its own BLK-5) exposes these two endpoints; this closes the
//! same gap for care-pathway. Today the only writer of this queue is the
//! bulk-import pipeline (`crate::bulk::pipeline::create_and_queue_for_review`,
//! `provenance = "import"`) — care-pathway has no batch `deduplicate`
//! scan of its own to route into it too (unlike case's own reference).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::MaybeAuthUser;
use crate::models::audit_logs::Model as AuditModel;

/// Review disposition of one queued duplicate pair, in the family's
/// lowercase wire tokens (matching person/worker/place/thing/organization
/// and case).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReviewStatus {
    /// Awaiting manual review.
    Pending,
    /// Confirmed as a duplicate — ready for merge.
    Confirmed,
    /// Rejected — not a duplicate.
    Rejected,
    /// Auto-merged (no auto-merge path here; present for wire parity).
    AutoMerged,
}

/// The lowercase wire token for a review status.
fn review_status_token(status: ReviewStatus) -> &'static str {
    match status {
        ReviewStatus::Pending => "pending",
        ReviewStatus::Confirmed => "confirmed",
        ReviewStatus::Rejected => "rejected",
        ReviewStatus::AutoMerged => "automerged",
    }
}

/// Parse a stored status token (unknown tokens read as `pending`).
fn parse_review_status(token: &str) -> ReviewStatus {
    match token {
        "confirmed" => ReviewStatus::Confirmed,
        "rejected" => ReviewStatus::Rejected,
        "automerged" => ReviewStatus::AutoMerged,
        _ => ReviewStatus::Pending,
    }
}

/// One stored review-queue item, in the family's person/worker shape
/// (`detection_method` included) so the front-ends render one queue.
#[derive(Debug, Serialize)]
struct ReviewQueueItem {
    /// Stable review-item id (survives re-scans).
    id: String,
    /// First care pathway in the candidate pair (public id).
    pathway_id_a: String,
    /// Second care pathway in the candidate pair (public id).
    pathway_id_b: String,
    /// Overall match score for the pair, in `[0.0, 1.0]`.
    match_score: f64,
    /// Confidence band label (matcher's `Confidence`, lowercased).
    match_quality: String,
    /// How the pair was detected.
    detection_method: String,
    /// Per-component score breakdown, as stored at detection time.
    score_breakdown: Option<serde_json::Value>,
    /// Current review state.
    status: ReviewStatus,
    /// How the pair was first surfaced (`operator` / `import` /
    /// `matcher_suggested`; §13 T-10).
    provenance: String,
    /// Reviewer identity recorded by the decision endpoint, if decided.
    reviewed_by: Option<String>,
    /// When the pair was first queued.
    created_at: chrono::DateTime<chrono::Utc>,
    /// When the decision was recorded, if decided.
    reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Map a stored review-queue row onto the wire item shape.
fn review_row_to_item(row: &crate::models::review_queue::ReviewQueueRow) -> ReviewQueueItem {
    ReviewQueueItem {
        id: row.id.to_string(),
        pathway_id_a: row.record_id_a.to_string(),
        pathway_id_b: row.record_id_b.to_string(),
        match_score: row.match_score,
        match_quality: row.match_quality.clone(),
        detection_method: row.detection_method.clone(),
        score_breakdown: row.score_breakdown.clone(),
        status: parse_review_status(&row.status),
        provenance: row.provenance.clone(),
        reviewed_by: row.reviewed_by.clone(),
        created_at: row.created_at,
        reviewed_at: row.reviewed_at,
    }
}

/// Query parameters for the review-queue list endpoint.
#[derive(Debug, Deserialize)]
struct ReviewQueueListQuery {
    /// Optional status-token filter (`pending` / `confirmed` /
    /// `rejected` / `automerged`).
    status: Option<String>,
    /// Maximum items to return (default 100, capped at 500).
    limit: Option<u64>,
}

/// Response for the review-queue list endpoint.
#[derive(Debug, Serialize)]
struct ReviewQueueListResponse {
    /// The stored review-queue items (newest first).
    items: Vec<ReviewQueueItem>,
    /// Number of items returned.
    total: usize,
}

/// List the stored duplicate review queue (newest first).
///
/// `GET /api/care-pathways/review-queue[?status=&limit=]`. An unknown
/// status token is `422`.
async fn get_review_queue(
    Query(query): Query<ReviewQueueListQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    if let Some(status) = query.status.as_deref()
        && !matches!(status, "pending" | "confirmed" | "rejected" | "automerged")
    {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new(
                "unprocessable_entity",
                format!("unknown review status `{status}`"),
            ),
        ));
    }
    let rows = crate::models::review_queue::list(
        &ctx.db,
        query.status.as_deref(),
        query.limit.unwrap_or(100),
    )
    .await?;
    let items: Vec<ReviewQueueItem> = rows.iter().map(review_row_to_item).collect();
    let total = items.len();
    format::json(ReviewQueueListResponse { items, total })
}

/// One operator verdict for a `pending` review item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ReviewDecision {
    /// Confirm the pair as a duplicate (ready for merge).
    Confirmed,
    /// Reject the pair (not a duplicate).
    Rejected,
}

/// Request body for the review-decision endpoint.
#[derive(Debug, Deserialize)]
struct ReviewDecisionRequest {
    /// The verdict (`confirmed` or `rejected`).
    status: ReviewDecision,
}

/// Decide one `pending` review item (`confirmed` or `rejected`).
///
/// `POST /api/care-pathways/review-queue/{id}/decision`. The transition
/// guard is first-writer-wins in SQL: only a `pending` item can be
/// decided; an already-decided item is `422`, an unknown id `404`. The
/// reviewer identity is the verified bearer `sub` when present, and each
/// decision writes a `review_decision` audit row.
async fn review_decision(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(id): Path<uuid::Uuid>,
    axum::Json(req): axum::Json<ReviewDecisionRequest>,
) -> Result<Response> {
    let token = review_status_token(match req.status {
        ReviewDecision::Confirmed => ReviewStatus::Confirmed,
        ReviewDecision::Rejected => ReviewStatus::Rejected,
    });
    let reviewed_by = caller.actor().map(str::to_string);
    match crate::models::review_queue::decide(&ctx.db, id, token, reviewed_by.as_deref()).await? {
        crate::models::review_queue::DecideOutcome::Decided(row) => {
            // A decision is a review-state mutation: record it on the
            // audit trail (best-effort, same posture as CRUD audits).
            if let Err(err) = AuditModel::record(
                &ctx.db,
                id,
                "review_decision",
                reviewed_by.as_deref(),
                Some(serde_json::json!({ "status": token })),
            )
            .await
            {
                tracing::warn!("review-decision audit write failed: {err}");
            }
            format::json(review_row_to_item(&row))
        }
        crate::models::review_queue::DecideOutcome::NotFound => Err(Error::NotFound),
        crate::models::review_queue::DecideOutcome::AlreadyDecided(current) => {
            Err(Error::CustomError(
                StatusCode::UNPROCESSABLE_ENTITY,
                ErrorDetail::new(
                    "unprocessable_entity",
                    format!("item is `{current}`; only `pending` items can be decided"),
                ),
            ))
        }
    }
}

/// The review-queue routes, mounted under `/api/care-pathways`. Kept
/// separate from `controllers::care_pathways::routes()` so this
/// controller's module boundary matches its own file; `app.rs` mounts
/// this before `care_pathways::routes()` so `/review-queue` is not
/// shadowed by the `/{pid}` capture.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/care-pathways")
        .add("/review-queue", get(get_review_queue))
        .add("/review-queue/{id}/decision", post(review_decision))
}
