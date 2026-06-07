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
