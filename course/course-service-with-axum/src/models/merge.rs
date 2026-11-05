//! Merge workflow — fold a duplicate Course into a surviving one.
//!
//! A merge keeps the `main` course and soft-deletes the `duplicate`,
//! transferring the duplicate's data onto main and recording the
//! operation as a [`MergeRecord`] for auditability and reversal.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::Course;

/// Inbound request body for `POST /api/courses/merge`.
///
/// Identifies the surviving course and the duplicate to fold into it,
/// with optional provenance for the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRequest {
    /// The surviving course that absorbs the duplicate's data.
    pub main_course_id: Uuid,
    /// The course to fold into main and soft-delete.
    pub duplicate_course_id: Uuid,
    /// Optional free-text reason recorded on the [`MergeRecord`].
    #[serde(default)]
    pub merge_reason: Option<String>,
    /// Optional actor identifier for the audit trail.
    #[serde(default)]
    pub merged_by: Option<String>,
}

/// Lifecycle state of a [`MergeRecord`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MergeStatus {
    /// The merge was applied; the duplicate is soft-deleted.
    Completed,
    /// A previously-completed merge was reversed.
    Reversed,
}

/// Persisted record of a single merge operation.
///
/// Stored in the `course_merge_records` table; surfaces the snapshot of
/// transferred data so a merge can be reviewed or reversed.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRecord {
    /// Server-generated record UUID.
    pub id: Uuid,
    /// The surviving course.
    pub main_course_id: Uuid,
    /// The course that was folded in and soft-deleted.
    pub duplicate_course_id: Uuid,
    /// Current lifecycle state of this merge.
    pub status: MergeStatus,
    /// Actor that performed the merge, if supplied.
    #[serde(default)]
    pub merged_by: Option<String>,
    /// Free-text reason, if supplied.
    #[serde(default)]
    pub merge_reason: Option<String>,
    /// Match score that motivated the merge, if it came from matching.
    #[serde(default)]
    pub match_score: Option<f64>,
    /// JSON snapshot of the fields transferred from duplicate to main,
    /// enabling review and reversal.
    #[serde(default)]
    pub transferred_data: Option<serde_json::Value>,
    /// When the merge was applied.
    pub merged_at: DateTime<Utc>,
}

/// Response body for a successful merge.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeResponse {
    /// The audit record for the operation.
    pub merge_record: MergeRecord,
    /// The surviving course after absorbing the duplicate.
    pub main_course: Course,
}
