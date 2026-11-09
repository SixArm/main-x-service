//! Models for the deduplication review queue.
//!
//! Batch deduplication ([`BatchDeduplicationRequest`] →
//! [`BatchDeduplicationResponse`]) scans the registry for potential duplicate
//! pairs. Pairs scoring above the auto-merge threshold are merged
//! automatically; the rest become [`ReviewQueueItem`]s with status
//! [`ReviewStatus::Pending`] for a human to confirm or reject. The
//! per-component score breakdown and the match evidence are preserved so
//! reviewers can see *why* a pair was flagged, and why it was or was not
//! marked auto-merged.
//!
//! A pair is marked [`ReviewStatus::AutoMerged`] only when its score clears
//! `auto_merge_threshold` **and** its evidence is `proven`
//! (`agents/share/match-evidence.md` §4); a `consistent` or `contradicted`
//! pair stays `pending` whatever its score.

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
    /// Auto-merged: score at or above the auto-merge threshold **and**
    /// evidence `proven`
    AutoMerged,
}

impl ReviewStatus {
    /// The status a batch scan gives a candidate pair: [`AutoMerged`](Self::AutoMerged)
    /// only when `score` clears `auto_merge_threshold` **and** the evidence
    /// permits an automatic merge (`proven`, `match-evidence.md` §4);
    /// otherwise [`Pending`](Self::Pending), whatever the score.
    #[must_use]
    pub fn for_scan(
        score: f64,
        auto_merge_threshold: f64,
        evidence: &crate::matching::MatchEvidence,
    ) -> Self {
        if score >= auto_merge_threshold && evidence.permits_auto_merge() {
            Self::AutoMerged
        } else {
            Self::Pending
        }
    }
}

/// An item in the deduplication review queue
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewQueueItem {
    /// Unique ID for this review item
    pub id: Uuid,

    /// First worker in the potential duplicate pair
    pub worker_id_a: Uuid,

    /// Second worker in the potential duplicate pair
    pub worker_id_b: Uuid,

    /// Match score between the two workers
    pub match_score: f64,

    /// Quality classification of the match
    pub match_quality: String,

    /// Which matching strategy detected this
    pub detection_method: String,

    /// Breakdown of individual score components
    pub score_breakdown: Option<serde_json::Value>,

    /// How much the compared data proves (`match-evidence.md`). Absent on
    /// items queued before evidence was recorded.
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

    /// Maximum number of candidates to evaluate per worker (default: 50)
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,

    /// Score at or above which a pair whose evidence is `proven` is marked
    /// auto-merged (default: 0.95). A pair short of proof stays `pending`
    /// whatever its score.
    #[serde(default = "default_auto_merge_threshold")]
    pub auto_merge_threshold: f64,
}

/// Serde default for [`BatchDeduplicationRequest::threshold`] (0.7): the
/// minimum score at which a pair is considered a potential duplicate.
fn default_threshold() -> f64 {
    0.7
}

/// Serde default for [`BatchDeduplicationRequest::max_candidates`] (50): the
/// cap on how many candidate workers each worker is compared against.
fn default_max_candidates() -> usize {
    50
}

/// Serde default for [`BatchDeduplicationRequest::auto_merge_threshold`]
/// (0.95): the score at or above which a pair is merged without human review.
fn default_auto_merge_threshold() -> f64 {
    0.95
}

/// Response from batch deduplication
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchDeduplicationResponse {
    /// Number of workers scanned
    pub workers_scanned: usize,

    /// Number of potential duplicates found
    pub duplicates_found: usize,

    /// Number marked auto-merged (score above the auto-merge threshold and
    /// evidence `proven`)
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

/// Request body for `POST /api/workers/review-queue/{id}/decision`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewDecisionRequest {
    /// The verdict (`confirmed` or `rejected`).
    pub status: ReviewDecision,
}

/// Response envelope for `GET /api/workers/review-queue`.
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
    use crate::matching::{MatchEvidence, MatchStrength};

    fn evidence(strength: MatchStrength) -> MatchEvidence {
        MatchEvidence {
            strength,
            ..MatchEvidence::default()
        }
    }

    /// Only a `proven` pair over the threshold is marked auto-merged; a
    /// `consistent` or `contradicted` one stays pending at any score.
    #[test]
    fn auto_merge_needs_score_and_proof() {
        let proven = evidence(MatchStrength::Proven);
        assert_eq!(
            ReviewStatus::for_scan(1.0, 0.95, &proven),
            ReviewStatus::AutoMerged
        );
        assert_eq!(
            ReviewStatus::for_scan(0.94, 0.95, &proven),
            ReviewStatus::Pending
        );
        for strength in [
            MatchStrength::Consistent,
            MatchStrength::Contradicted,
            MatchStrength::NonMatch,
        ] {
            assert_eq!(
                ReviewStatus::for_scan(1.0, 0.95, &evidence(strength)),
                ReviewStatus::Pending,
                "{strength:?}"
            );
        }
    }

    /// An item serialized before `evidence` existed still deserialises,
    /// with no evidence.
    #[test]
    fn an_item_without_evidence_still_deserialises() {
        let old = serde_json::json!({
            "id": Uuid::nil(),
            "worker_id_a": Uuid::nil(),
            "worker_id_b": Uuid::nil(),
            "match_score": 0.9,
            "match_quality": "certain",
            "detection_method": "batch_deduplication",
            "score_breakdown": { "name_score": 1.0 },
            "status": "pending",
            "reviewed_by": null,
            "created_at": "2026-07-19T00:00:00Z",
            "reviewed_at": null,
        });
        let item: ReviewQueueItem = serde_json::from_value(old).unwrap();
        assert!(item.evidence.is_none());
    }

    /// Stored evidence round-trips through the item.
    #[test]
    fn an_item_with_evidence_round_trips() {
        let evidence = crate::matching::MatchEvidence {
            weight_bits: 12.5,
            fields_compared: 3,
            strength: crate::matching::MatchStrength::Consistent,
            ..Default::default()
        };
        let stored = serde_json::to_value(&evidence).unwrap();
        let value = serde_json::json!({
            "id": Uuid::nil(),
            "worker_id_a": Uuid::nil(),
            "worker_id_b": Uuid::nil(),
            "match_score": 0.9,
            "match_quality": "certain",
            "detection_method": "batch_deduplication",
            "score_breakdown": null,
            "evidence": stored,
            "status": "pending",
            "reviewed_by": null,
            "created_at": "2026-07-19T00:00:00Z",
            "reviewed_at": null,
        });
        let item: ReviewQueueItem = serde_json::from_value(value).unwrap();
        assert_eq!(item.evidence, Some(evidence));
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
