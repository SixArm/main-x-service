//! Record-merge models for the deduplication workflow.
//!
//! When two records are confirmed to be the same person, one is chosen
//! as the surviving **main** record and the other as the **duplicate**.
//! [`MergeRequest`] drives the operation, [`MergeRecord`] is the
//! persisted audit trail (including a JSON snapshot of transferred
//! data), and [`MergeResponse`] returns both the record and the updated
//! main person. See the merge handler in
//! [`crate::api::rest::handlers`] for the full flow.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Status of a merge operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MergeStatus {
    /// Merge completed successfully
    Completed,
    /// Merge was reversed/undone
    Reversed,
}

/// Record of a person merge operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRecord {
    /// Unique merge operation identifier
    pub id: Uuid,

    /// The main/surviving person record ID
    pub main_person_id: Uuid,

    /// The duplicate/merged person record ID (now inactive)
    pub duplicate_person_id: Uuid,

    /// Status of this merge
    pub status: MergeStatus,

    /// User who performed the merge
    pub merged_by: Option<String>,

    /// Reason for the merge
    pub merge_reason: Option<String>,

    /// Match score that triggered the merge review
    pub match_score: Option<f64>,

    /// Snapshot of data transferred from duplicate to main
    pub transferred_data: Option<serde_json::Value>,

    /// When the merge was performed
    pub merged_at: Timestamp,
}

/// Request to merge two person records
#[derive(Debug, Deserialize, ToSchema)]
pub struct MergeRequest {
    /// The main/surviving person ID
    pub main_person_id: Uuid,

    /// The duplicate person ID to merge into main
    pub duplicate_person_id: Uuid,

    /// Reason for the merge
    pub merge_reason: Option<String>,

    /// User performing the merge
    pub merged_by: Option<String>,
}

/// Response after a merge operation
#[derive(Debug, Serialize, ToSchema)]
pub struct MergeResponse {
    /// The merge record
    pub merge_record: MergeRecord,

    /// The updated main person record
    pub main_person: super::Person,
}
