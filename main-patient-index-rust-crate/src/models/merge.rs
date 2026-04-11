//! Record merge models

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

/// Record of a patient merge operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRecord {
    /// Unique merge operation identifier
    pub id: Uuid,

    /// The master/surviving patient record ID
    pub main_patient_id: Uuid,

    /// The duplicate/merged patient record ID (now inactive)
    pub duplicate_patient_id: Uuid,

    /// Status of this merge
    pub status: MergeStatus,

    /// User who performed the merge
    pub merged_by: Option<String>,

    /// Reason for the merge
    pub merge_reason: Option<String>,

    /// Match score that triggered the merge review
    pub match_score: Option<f64>,

    /// Snapshot of data transferred from duplicate to master
    pub transferred_data: Option<serde_json::Value>,

    /// When the merge was performed
    pub merged_at: DateTime<Utc>,
}

/// Request to merge two patient records
#[derive(Debug, Deserialize, ToSchema)]
pub struct MergeRequest {
    /// The master/surviving patient ID
    pub main_patient_id: Uuid,

    /// The duplicate patient ID to merge into master
    pub duplicate_patient_id: Uuid,

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

    /// The updated main patient record
    pub main_patient: super::Patient,
}
