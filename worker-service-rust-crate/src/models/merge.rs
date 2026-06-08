//! Models for the worker record-merge workflow.
//!
//! When two worker records are confirmed duplicates, one is chosen as the
//! surviving *main* record and the other becomes the *duplicate*. This module
//! carries the three shapes that flow through that operation:
//!
//! - [`MergeRequest`] — the inbound API request (which two records, why).
//! - [`MergeRecord`] — the persisted audit record of a merge, including a JSON
//!   snapshot of the data transferred from duplicate to main.
//! - [`MergeResponse`] — the outbound API response (the record plus the merged
//!   main worker).
//!
//! [`MergeStatus`] tracks whether a merge is [`Completed`](MergeStatus::Completed)
//! or was later [`Reversed`](MergeStatus::Reversed).

use jiff::Timestamp;
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

/// Record of a worker merge operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRecord {
    /// Unique merge operation identifier
    pub id: Uuid,

    /// The main/surviving worker record ID
    pub main_worker_id: Uuid,

    /// The duplicate/merged worker record ID (now inactive)
    pub duplicate_worker_id: Uuid,

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

/// Request to merge two worker records
#[derive(Debug, Deserialize, ToSchema)]
pub struct MergeRequest {
    /// The main/surviving worker ID
    pub main_worker_id: Uuid,

    /// The duplicate worker ID to merge into main
    pub duplicate_worker_id: Uuid,

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

    /// The updated main worker record
    pub main_worker: super::Worker,
}
