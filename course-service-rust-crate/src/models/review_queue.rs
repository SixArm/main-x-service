//! Deduplication review queue.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ReviewStatus {
    Pending,
    Confirmed,
    Rejected,
    AutoMerged,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewQueueItem {
    pub id: Uuid,
    pub course_id_a: Uuid,
    pub course_id_b: Uuid,
    pub match_score: f64,
    pub match_quality: String,
    pub detection_method: String,
    #[serde(default)]
    pub score_breakdown: Option<serde_json::Value>,
    pub status: ReviewStatus,
    #[serde(default)]
    pub reviewed_by: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchDeduplicationRequest {
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: u32,
    #[serde(default = "default_auto_merge_threshold")]
    pub auto_merge_threshold: f64,
}

fn default_threshold() -> f64 {
    0.70
}
fn default_max_candidates() -> u32 {
    50
}
fn default_auto_merge_threshold() -> f64 {
    0.95
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BatchDeduplicationResponse {
    pub courses_scanned: u64,
    pub duplicates_found: u64,
    pub auto_merged: u64,
    pub queued_for_review: u64,
    pub review_items: Vec<ReviewQueueItem>,
}
