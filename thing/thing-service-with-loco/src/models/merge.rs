//! Merge request/response/record types for folding duplicate things.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::thing::Thing;

/// Request to merge a duplicate thing into a surviving main thing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRequest {
    /// The surviving thing id.
    pub main_thing_id: Uuid,
    /// The duplicate thing id (will be soft-deleted).
    pub duplicate_thing_id: Uuid,
    /// Free-text reason for the merge.
    #[serde(default)]
    pub merge_reason: Option<String>,
}

/// Persisted record of a completed merge.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRecord {
    /// Unique merge-record id.
    pub id: Uuid,
    /// The surviving thing id.
    pub main_thing_id: Uuid,
    /// The absorbed (soft-deleted) thing id.
    pub duplicate_thing_id: Uuid,
    /// Free-text reason for the merge.
    pub merge_reason: Option<String>,
    /// Snapshot of data transferred from duplicate to main.
    pub transferred_data: Option<serde_json::Value>,
    /// When the merge happened.
    pub merged_at: DateTime<Utc>,
}

/// Response returned by the merge endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeResponse {
    /// The persisted merge record.
    pub merge_record: MergeRecord,
    /// The surviving thing after the merge.
    pub main_thing: Thing,
}
