//! Deduplication review queue models.
//!
//! Batch deduplication ([`BatchDeduplicationRequest`] →
//! [`BatchDeduplicationResponse`]) scans the index for candidate
//! duplicate pairs. High-confidence pairs may be auto-merged; the rest
//! become [`ReviewQueueItem`]s awaiting a human decision
//! ([`ReviewStatus`]). The default thresholds are supplied by the
//! private `default_*` helpers below.
//!
//! The queue is persisted (spec §13 T-15) in the `review_queue` table
//! ([`crate::db::review_queue`]), listed by `GET /api/events/review-queue`
//! and decided by `POST /api/events/review-queue/{id}/decision`. A pair is
//! merged and marked `AutoMerged` only when its score clears
//! `auto_merge_threshold` **and** its evidence is `Proven`
//! (`agents/share/match-evidence.md` §4); `Consistent` and `Contradicted`
//! pairs stay `Pending`.

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
    /// Merged by the batch scan: score at or above the auto-merge
    /// threshold **and** evidence `proven` (T-15).
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

    /// The matcher's evidence for the pair (`match-evidence.md`). Absent
    /// on items serialised before it existed.
    #[serde(default)]
    pub evidence: Option<crate::matching::MatchEvidence>,

    /// Current review status
    pub status: ReviewStatus,

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

    /// Maximum number of candidates to evaluate per event (default: 50)
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,

    /// Score at or above which a **proven** pair is marked auto-merged
    /// (default: 0.95); a pair short of proof stays pending whatever its
    /// score
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

    /// Number merged by the scan (score at or above the auto-merge
    /// threshold and evidence proven)
    pub auto_merged: usize,

    /// Number of proven pairs whose merge failed (an event already merged
    /// away earlier in the scan, a concurrent merge, a database error);
    /// each stays `pending` for a person
    pub auto_merge_failed: usize,

    /// Number added to review queue
    pub queued_for_review: usize,

    /// The stored review-queue rows for the pairs found (ids are stable
    /// across scans; a decided pair keeps its decision)
    pub review_items: Vec<ReviewQueueItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An item serialised before `evidence` existed still deserialises,
    /// with no evidence.
    #[test]
    fn item_without_evidence_still_deserialises() {
        let old = serde_json::json!({
            "id": Uuid::nil(),
            "event_id_a": Uuid::nil(),
            "event_id_b": Uuid::nil(),
            "match_score": 0.9,
            "match_quality": "certain",
            "detection_method": "batch_deduplication",
            "score_breakdown": null,
            "status": "pending",
            "reviewed_by": null,
            "created_at": "2026-11-01T00:00:00Z",
            "reviewed_at": null
        });
        let item: ReviewQueueItem = serde_json::from_value(old).unwrap();
        assert!(item.evidence.is_none());
        assert_eq!(item.status, ReviewStatus::Pending);
    }

    /// Evidence round-trips on an item.
    #[test]
    fn item_with_evidence_round_trips() {
        let json = serde_json::json!({
            "id": Uuid::nil(),
            "event_id_a": Uuid::nil(),
            "event_id_b": Uuid::nil(),
            "match_score": 0.9,
            "match_quality": "certain",
            "detection_method": "batch_deduplication",
            "score_breakdown": null,
            "evidence": { "weight_bits": 17.4, "fields_compared": 2, "strength": "consistent" },
            "status": "pending",
            "reviewed_by": null,
            "created_at": "2026-11-01T00:00:00Z",
            "reviewed_at": null
        });
        let item: ReviewQueueItem = serde_json::from_value(json).unwrap();
        let e = item.evidence.unwrap();
        assert_eq!(
            e.strength,
            crate::matching::evidence::EvidenceStrength::Consistent
        );
        assert!(e.contradictions.is_empty() && e.components.is_empty());
    }
}
