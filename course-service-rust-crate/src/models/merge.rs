//! Merge workflow — fold a duplicate Course into a surviving one.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::Course;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRequest {
    pub main_course_id: Uuid,
    pub duplicate_course_id: Uuid,
    #[serde(default)]
    pub merge_reason: Option<String>,
    #[serde(default)]
    pub merged_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MergeStatus {
    Completed,
    Reversed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeRecord {
    pub id: Uuid,
    pub main_course_id: Uuid,
    pub duplicate_course_id: Uuid,
    pub status: MergeStatus,
    #[serde(default)]
    pub merged_by: Option<String>,
    #[serde(default)]
    pub merge_reason: Option<String>,
    #[serde(default)]
    pub match_score: Option<f64>,
    #[serde(default)]
    pub transferred_data: Option<serde_json::Value>,
    pub merged_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeResponse {
    pub merge_record: MergeRecord,
    pub main_course: Course,
}
