//! Record merge models.
//!
//! When two events are confirmed duplicates, one survives (the *main*
//! record) and the other is soft-deleted. A [`MergeRequest`] drives the
//! operation, a [`MergeResponse`] reports the result, and a
//! [`MergeRecord`] is persisted as an audit-able history entry holding a
//! snapshot of the transferred data. See `agents/share/merge.md` for
//! the workflow.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

/// Status of a merge operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MergeStatus {
    /// Merge completed successfully
    Completed,
    /// Merge was reversed/undone
    Reversed,
}

/// Record of a event merge operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRecord {
    /// Unique merge operation identifier
    pub id: Uuid,

    /// The main/surviving event record ID
    pub main_event_id: Uuid,

    /// The duplicate/merged event record ID (now inactive)
    pub duplicate_event_id: Uuid,

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
    pub merged_at: DateTime<Utc>,
}

/// Request to merge two event records
#[derive(Debug, Deserialize, ToSchema)]
pub struct MergeRequest {
    /// The main/surviving event ID
    pub main_event_id: Uuid,

    /// The duplicate event ID to merge into main
    pub duplicate_event_id: Uuid,

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

    /// The updated main event record
    pub main_event: crate::models::Event,
}
