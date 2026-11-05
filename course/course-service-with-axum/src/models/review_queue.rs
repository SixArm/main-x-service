//! Deduplication review queue.
//!
//! When duplicate detection finds candidate pairs below the auto-merge
//! threshold, it captures them as [`ReviewQueueItem`]s for a human to
//! confirm or reject. A batch scan
//! ([`BatchDeduplicationRequest`]/[`BatchDeduplicationResponse`]) walks
//! the whole index and either auto-merges high-confidence pairs or
//! queues the rest.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Lifecycle state of a [`ReviewQueueItem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ReviewStatus {
    /// Awaiting human review. The default for a freshly-queued pair.
    Pending,
    /// A reviewer confirmed the pair are duplicates.
    Confirmed,
    /// A reviewer rejected the pair as distinct.
    Rejected,
    /// The pair scored above the auto-merge threshold and was merged
    /// without human review.
    AutoMerged,
}

impl ReviewStatus {
    /// `true` for the two verdicts an operator may record via
    /// `POST /api/courses/review-queue/{id}/decision` (T-27).
    /// `Pending` is the pre-decision state and `AutoMerged` is only ever
    /// reached by the batch scan itself, so neither is a valid inbound
    /// decision.
    #[must_use]
    pub fn is_decision(self) -> bool {
        matches!(self, Self::Confirmed | Self::Rejected)
    }
}

/// Order a pair of course ids deterministically (smaller first), so an
/// unordered `(a, b)` pair has one canonical key regardless of which
/// side was probed first. Shared by the batch-dedup scan's in-memory
/// `seen_pairs` set and the persisted review queue's upsert key (T-27),
/// so both agree on the same pair identity.
#[must_use]
pub fn canonical_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b { (a, b) } else { (b, a) }
}

/// A candidate duplicate pair captured for review.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewQueueItem {
    /// Server-generated queue-item UUID.
    pub id: Uuid,
    /// First course in the candidate pair.
    pub course_id_a: Uuid,
    /// Second course in the candidate pair.
    pub course_id_b: Uuid,
    /// Overall match score for the pair, in `[0.0, 1.0]`.
    pub match_score: f64,
    /// Human-readable confidence band (e.g. "probable").
    pub match_quality: String,
    /// How the pair was detected (e.g. "batch", "on-create").
    pub detection_method: String,
    /// Optional per-component score breakdown as JSON.
    #[serde(default)]
    pub score_breakdown: Option<serde_json::Value>,
    /// Current review state.
    pub status: ReviewStatus,
    /// Actor that reviewed the item, once reviewed.
    #[serde(default)]
    pub reviewed_by: Option<String>,
    /// When the item was queued.
    pub created_at: DateTime<Utc>,
    /// When the item was reviewed, if it has been.
    #[serde(default)]
    pub reviewed_at: Option<DateTime<Utc>>,
}

/// Inbound tuning for a full-index deduplication scan.
///
/// All fields default via the `default_*` helpers below when omitted.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchDeduplicationRequest {
    /// Minimum score for a pair to count as a candidate duplicate.
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    /// Cap on candidate comparisons considered per course.
    #[serde(default = "default_max_candidates")]
    pub max_candidates: u32,
    /// Score at or above which a pair is auto-merged without review.
    #[serde(default = "default_auto_merge_threshold")]
    pub auto_merge_threshold: f64,
}

/// Default candidate threshold (0.70) when the request omits it.
fn default_threshold() -> f64 {
    0.70
}
/// Default per-course candidate cap (50) when the request omits it.
fn default_max_candidates() -> u32 {
    50
}
/// Default auto-merge threshold (0.95) when the request omits it.
fn default_auto_merge_threshold() -> f64 {
    0.95
}

/// A candidate pair to insert-or-refresh into the persisted review
/// queue (T-27, `course_match_scores`). Pair order is normalized
/// internally ([`canonical_pair`]) so `(a, b)` and `(b, a)` upsert the
/// same row, and a re-scan refreshes the score columns while leaving a
/// previously-decided row's `status` untouched.
#[derive(Debug, Clone)]
pub struct NewReviewItem {
    /// First course of the pair (order-insensitive; normalized on write).
    pub course_id_a: Uuid,
    /// Second course of the pair.
    pub course_id_b: Uuid,
    /// Overall match score in `[0.0, 1.0]`.
    pub match_score: f64,
    /// Confidence band label.
    pub match_quality: String,
    /// How the pair was detected.
    pub detection_method: String,
    /// Optional per-component score breakdown.
    pub score_breakdown: Option<serde_json::Value>,
}

/// Outcome of a decision attempt on one persisted review item (T-27).
/// The transition guard lives in the storage layer's `WHERE status =
/// 'Pending'` update, so concurrent decisions cannot double-apply —
/// exactly one caller observes [`Self::Decided`], the rest observe
/// [`Self::AlreadyDecided`].
#[derive(Debug, Clone)]
pub enum DecideOutcome {
    /// The row moved from `Pending` to the requested status.
    Decided(Box<ReviewQueueItem>),
    /// No row with that id exists.
    NotFound,
    /// The row exists but is not `Pending`; carries its current status.
    AlreadyDecided(ReviewStatus),
}

/// Request body for `POST /api/courses/review-queue/{id}/decision`.
///
/// Only [`ReviewStatus::Confirmed`] and [`ReviewStatus::Rejected`] are
/// accepted verdicts — the handler rejects anything else (checked via
/// [`ReviewStatus::is_decision`]) as `422`. Reuses `ReviewStatus`
/// directly rather than a parallel decision-only enum, so the wire
/// tokens can never drift from the ones `ReviewQueueItem::status`
/// already publishes.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReviewDecisionRequest {
    /// The verdict — must be `Confirmed` or `Rejected`.
    pub status: ReviewStatus,
    /// Reviewer identity to record, client-supplied — mirrors
    /// [`crate::models::MergeRequest::merged_by`], this crate's existing
    /// convention for actor attribution (no `MaybeAuthUser` extractor
    /// exists here to derive it from a bearer token).
    #[serde(default)]
    pub reviewed_by: Option<String>,
}

/// Summary of a completed deduplication scan.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchDeduplicationResponse {
    /// Total courses examined.
    pub courses_scanned: u64,
    /// Candidate duplicate pairs found at or above the threshold.
    pub duplicates_found: u64,
    /// Pairs auto-merged without human review.
    pub auto_merged: u64,
    /// Pairs queued for human review.
    pub queued_for_review: u64,
    /// The queued items themselves.
    pub review_items: Vec<ReviewQueueItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only `Confirmed`/`Rejected` are valid inbound decisions (T-27);
    /// `Pending` is the pre-decision state and `AutoMerged` is reachable
    /// only via the batch scan itself.
    #[test]
    fn is_decision_accepts_only_confirmed_or_rejected() {
        assert!(ReviewStatus::Confirmed.is_decision());
        assert!(ReviewStatus::Rejected.is_decision());
        assert!(!ReviewStatus::Pending.is_decision());
        assert!(!ReviewStatus::AutoMerged.is_decision());
    }

    /// `ReviewStatus` carries no `rename_all`, so its wire form is the
    /// bare variant name — matching the `course_match_scores` migration's
    /// `DEFAULT 'Pending'` column default. A silent switch to a lowercase
    /// token (mirroring other family members) would break both the SQL
    /// default and the `OpenAPI` schema already published for this crate.
    #[test]
    fn review_status_wire_tokens_are_pascal_case() {
        assert_eq!(
            serde_json::to_value(ReviewStatus::Pending).unwrap(),
            serde_json::json!("Pending")
        );
        assert_eq!(
            serde_json::to_value(ReviewStatus::Confirmed).unwrap(),
            serde_json::json!("Confirmed")
        );
        assert_eq!(
            serde_json::to_value(ReviewStatus::Rejected).unwrap(),
            serde_json::json!("Rejected")
        );
        assert_eq!(
            serde_json::to_value(ReviewStatus::AutoMerged).unwrap(),
            serde_json::json!("AutoMerged")
        );
    }

    /// `reviewed_by` is optional on the wire (omitted ⇒ `None`), matching
    /// [`MergeRequest::merged_by`](crate::models::MergeRequest)'s
    /// client-supplied-actor convention.
    #[test]
    fn review_decision_request_defaults_reviewed_by_when_omitted() {
        let req: ReviewDecisionRequest =
            serde_json::from_value(serde_json::json!({ "status": "Confirmed" })).unwrap();
        assert_eq!(req.status, ReviewStatus::Confirmed);
        assert_eq!(req.reviewed_by, None);
    }
}
