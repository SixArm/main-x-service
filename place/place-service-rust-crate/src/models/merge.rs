//! Merge request/response/record types for folding duplicate places.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::place::Place;

/// Request to merge a duplicate place into a surviving main place.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRequest {
    /// The surviving place id.
    pub main_place_id: Uuid,
    /// The duplicate place id (will be soft-deleted).
    pub duplicate_place_id: Uuid,
    /// Free-text reason for the merge.
    #[serde(default)]
    pub merge_reason: Option<String>,
}

/// Persisted record of a completed merge.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRecord {
    /// Unique merge-record id.
    pub id: Uuid,
    /// The surviving place id.
    pub main_place_id: Uuid,
    /// The absorbed (soft-deleted) place id.
    pub duplicate_place_id: Uuid,
    /// Free-text reason for the merge.
    pub merge_reason: Option<String>,
    /// Snapshot of data transferred from duplicate to main.
    pub transferred_data: Option<serde_json::Value>,
    /// When the merge happened.
    pub merged_at: Timestamp,
}

/// Response returned by the merge endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeResponse {
    /// The persisted merge record.
    pub merge_record: MergeRecord,
    /// The surviving place after the merge.
    pub main_place: Place,
}
