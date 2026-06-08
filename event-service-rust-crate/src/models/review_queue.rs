//! Deduplication review queue models.
//!
//! Batch deduplication ([`BatchDeduplicationRequest`] →
//! [`BatchDeduplicationResponse`]) scans the index for candidate
//! duplicate pairs. High-confidence pairs may be auto-merged; the rest
//! become [`ReviewQueueItem`]s awaiting a human decision
//! ([`ReviewStatus`]). The default thresholds are supplied by the
//! private `default_*` helpers below.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

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

    /// First event in the potential duplicate pair
    pub event_id_a: Uuid,

    /// Second event in the potential duplicate pair
    pub event_id_b: Uuid,

    /// Match score between the two events
    pub match_score: f64,

    /// Quality classification of the match
    pub match_quality: String,

    /// Which matching strategy detected this
    pub detection_method: String,

    /// Breakdown of individual score components
    pub score_breakdown: Option<serde_json::Value>,

    /// Current review status
    pub status: ReviewStatus,

    /// User who reviewed this item
    pub reviewed_by: Option<String>,

    /// When this item was created
    pub created_at: Timestamp,

    /// When this item was last reviewed
    pub reviewed_at: Option<Timestamp>,
}

/// Request to run batch deduplication
#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchDeduplicationRequest {
    /// Minimum match score threshold (default: 0.7)
    #[serde(default = "default_threshold")]
    pub threshold: f64,

    /// Maximum number of candidates to evaluate per event (default: 50)
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,

    /// Score above which duplicates are auto-merged (default: 0.95)
    #[serde(default = "default_auto_merge_threshold")]
    pub auto_merge_threshold: f64,
}

/// serde default for [`BatchDeduplicationRequest::threshold`].
fn default_threshold() -> f64 {
    0.7
}

/// serde default for [`BatchDeduplicationRequest::max_candidates`].
fn default_max_candidates() -> usize {
    50
}

/// serde default for [`BatchDeduplicationRequest::auto_merge_threshold`].
fn default_auto_merge_threshold() -> f64 {
    0.95
}

/// Response from batch deduplication
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchDeduplicationResponse {
    /// Number of events scanned
    pub events_scanned: usize,

    /// Number of potential duplicates found
    pub duplicates_found: usize,

    /// Number auto-merged (above auto-merge threshold)
    pub auto_merged: usize,

    /// Number added to review queue
    pub queued_for_review: usize,

    /// The review queue items created
    pub review_items: Vec<ReviewQueueItem>,
}
