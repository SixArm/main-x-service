//! Deduplication review-queue models.
//!
//! A batch deduplication scan ([`crate::api::rest::handlers`]) compares
//! active persons pairwise and emits a [`ReviewQueueItem`] for each pair
//! scoring above the configured [`threshold`](BatchDeduplicationRequest::threshold).
//! Pairs above the [`auto_merge_threshold`](BatchDeduplicationRequest::auto_merge_threshold)
//! are flagged [`ReviewStatus::AutoMerged`]; the rest are
//! [`ReviewStatus::Pending`] for a human to confirm or reject.
//! [`BatchDeduplicationRequest`] / [`BatchDeduplicationResponse`] are
//! the request/response envelope for that scan.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Status of a review queue item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewStatus {
    /// Awaiting manual review
    Pending,
    /// Confirmed as duplicate — ready for merge
    Confirmed,
    /// Rejected — not a duplicate
    Rejected,
    /// Auto-merged (score above auto-merge threshold)
    AutoMerged,
}

/// An item in the deduplication review queue
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewQueueItem {
    /// Unique ID for this review item
    pub id: Uuid,

    /// First person in the potential duplicate pair
    pub person_id_a: Uuid,

    /// Second person in the potential duplicate pair
    pub person_id_b: Uuid,

    /// Match score between the two persons
    pub match_score: f64,

    /// Quality classification of the match
    pub match_quality: String,

    /// Which matching strategy detected this
    pub detection_method: String,

    /// Breakdown of individual score components
    pub score_breakdown: Option<serde_json::Value>,

    /// Current review status
    pub status: ReviewStatus,

    /// How this pair was first surfaced (`operator` / `import` /
    /// `matcher_suggested`).
    pub provenance: String,

    /// User who reviewed this item
    pub reviewed_by: Option<String>,

    /// When this item was created
    pub created_at: DateTime<Utc>,

    /// When this item was last reviewed
    pub reviewed_at: Option<DateTime<Utc>>,
}

/// Request to run batch deduplication
#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchDeduplicationRequest {
    /// Minimum match score threshold (default: 0.7)
    #[serde(default = "default_threshold")]
    pub threshold: f64,

    /// Maximum number of candidates to evaluate per person (default: 50)
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,

    /// Score above which duplicates are auto-merged (default: 0.95)
    #[serde(default = "default_auto_merge_threshold")]
    pub auto_merge_threshold: f64,
}

/// serde default for [`BatchDeduplicationRequest::threshold`] (0.7).
fn default_threshold() -> f64 {
    0.7
}

/// serde default for [`BatchDeduplicationRequest::max_candidates`] (50).
fn default_max_candidates() -> usize {
    50
}

/// serde default for [`BatchDeduplicationRequest::auto_merge_threshold`] (0.95).
fn default_auto_merge_threshold() -> f64 {
    0.95
}

/// Response from batch deduplication
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchDeduplicationResponse {
    /// Number of persons scanned
    pub persons_scanned: usize,

    /// Number of potential duplicates found
    pub duplicates_found: usize,

    /// Number auto-merged (above auto-merge threshold)
    pub auto_merged: usize,

    /// Number added to review queue
    pub queued_for_review: usize,

    /// The review queue items created
    pub review_items: Vec<ReviewQueueItem>,
}

/// One operator verdict for a `pending` review item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReviewDecision {
    /// Confirm the pair as a duplicate (ready for merge).
    Confirmed,
    /// Reject the pair (not a duplicate).
    Rejected,
}

/// Request body for `POST /api/persons/review-queue/{id}/decision`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewDecisionRequest {
    /// The verdict (`confirmed` or `rejected`).
    pub status: ReviewDecision,
}

/// Confidence-band label for a match score: `certain` (≥ 0.95),
/// `probable` (≥ 0.7), else `possible`. Extracted so the thresholds live
/// in exactly one place — reused by the batch dedup scan
/// (`api::rest::handlers::deduplicate`) and the T-32 cross-service
/// `same_identity` suggestion review-queue write
/// (`api::rest::links::create_link`), which previously would have had to
/// duplicate this inline `if`/`else` a second time.
#[must_use]
pub fn match_quality_for_score(score: f64) -> &'static str {
    if score >= 0.95 {
        "certain"
    } else if score >= 0.7 {
        "probable"
    } else {
        "possible"
    }
}

/// Response envelope for `GET /api/persons/review-queue`.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReviewQueueListResponse {
    /// The stored review-queue items (newest first).
    pub items: Vec<ReviewQueueItem>,
    /// Number of items returned.
    pub total: usize,
}

#[cfg(test)]
mod review_decision_tests {
    use super::*;

    /// `match_quality_for_score`'s three bands, pinned at their boundary
    /// values (both inclusive-at, and just-below).
    #[test]
    fn match_quality_bands() {
        assert_eq!(match_quality_for_score(1.0), "certain");
        assert_eq!(match_quality_for_score(0.95), "certain");
        assert_eq!(match_quality_for_score(0.9499), "probable");
        assert_eq!(match_quality_for_score(0.7), "probable");
        assert_eq!(match_quality_for_score(0.6999), "possible");
        assert_eq!(match_quality_for_score(0.0), "possible");
    }

    /// Decision tokens are the lowercase wire form, and only the two
    /// operator verdicts parse — `pending` / `automerged` are refused.
    #[test]
    fn decision_wire_tokens() {
        let ok: ReviewDecisionRequest =
            serde_json::from_value(serde_json::json!({"status": "confirmed"})).unwrap();
        assert_eq!(ok.status, ReviewDecision::Confirmed);
        let ok: ReviewDecisionRequest =
            serde_json::from_value(serde_json::json!({"status": "rejected"})).unwrap();
        assert_eq!(ok.status, ReviewDecision::Rejected);
        assert!(
            serde_json::from_value::<ReviewDecisionRequest>(
                serde_json::json!({"status": "pending"})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<ReviewDecisionRequest>(
                serde_json::json!({"status": "automerged"})
            )
            .is_err()
        );
    }
}
