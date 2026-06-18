//! `CourseInstance` — schema.org/CourseInstance.
//!
//! A specific offering of a `Course` at a particular time / place /
//! mode (e.g. CS101 Fall 2026 with Prof. Smith). Multiple instances
//! can share the same parent `Course`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A specific scheduled offering of a [`Course`](crate::models::course::Course).
///
/// Persisted in the `course_instances` table and surfaced under the
/// `/api/courses/{id}/instances` sub-resource. Constructed from raw
/// input without validation; `crate::validation::validate_instance`
/// enforces FR-24 + FR-26..FR-28.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CourseInstance {
    /// Server-generated UUID. Defaults to a fresh v4 on missing input.
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// FK back to the owning `Course`.
    pub course_id: Uuid,

    /// Human-readable name (often "{course.name} — {term}").
    #[serde(default)]
    pub name: Option<String>,
    /// schema.org/courseMode.
    #[serde(default)]
    pub course_mode: Option<CourseMode>,
    /// Lifecycle state of this specific offering.
    #[serde(default)]
    pub status: CourseInstanceStatus,

    /// Schedule for this offering — start / end / sessions / etc.
    #[serde(default)]
    pub schedule: Option<Schedule>,

    /// schema.org/inLanguage for this instance (may differ from parent).
    #[serde(default)]
    pub in_language: Vec<String>,
    /// schema.org/location — free-text, URL, or external place-service ID.
    #[serde(default)]
    pub location: Option<String>,
    /// External place-service reference (preferred over free-text).
    #[serde(default)]
    pub location_id: Option<Uuid>,
    /// schema.org/instructor — external person-service IDs.
    #[serde(default)]
    pub instructor_ids: Vec<Uuid>,
    /// Free-text instructor names when an external reference is not available.
    #[serde(default)]
    pub instructor_names: Vec<String>,

    /// schema.org/maximumAttendeeCapacity.
    #[serde(default)]
    pub maximum_attendee_capacity: Option<u32>,
    /// Enrollment so far for this instance.
    #[serde(default)]
    pub enrolled_count: Option<u32>,

    /// Enrollment window opens — ISO 8601 in UTC. Validated to be ≤
    /// [`enrollment_closes`](Self::enrollment_closes) (FR-27).
    #[serde(default)]
    pub enrollment_opens: Option<DateTime<Utc>>,
    /// Enrollment window closes — ISO 8601 in UTC.
    #[serde(default)]
    pub enrollment_closes: Option<DateTime<Utc>>,

    /// Created server-side on insert.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Updated server-side on insert and update.
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

/// schema.org/CourseInstance.courseMode — how the offering is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CourseMode {
    /// Delivered fully online.
    Online,
    /// Delivered in person at a physical location.
    Onsite,
    /// Mix of online and onsite delivery.
    Blended,
    /// Learner-paced, no fixed schedule.
    SelfPaced,
}

/// Lifecycle state of a single [`CourseInstance`] offering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CourseInstanceStatus {
    /// Planned but enrollment has not yet opened. The default state.
    #[default]
    Scheduled,
    /// Currently accepting enrollments.
    EnrollmentOpen,
    /// Enrollment window has closed; offering not yet started.
    EnrollmentClosed,
    /// Offering is underway.
    InProgress,
    /// Offering has finished.
    Completed,
    /// Offering was cancelled.
    Cancelled,
}

/// Time window for a course instance. Either a single
/// `start_date`/`end_date` window or an explicit list of session
/// times; both are optional so this also accommodates self-paced
/// offerings.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Schedule {
    /// Window start. Validated to be ≤ [`end_date`](Self::end_date)
    /// when both are present (FR-26).
    #[serde(default)]
    pub start_date: Option<DateTime<Utc>>,
    /// Window end.
    #[serde(default)]
    pub end_date: Option<DateTime<Utc>>,
    /// IANA tz string (storage is UTC).
    #[serde(default)]
    pub time_zone: Option<String>,
    /// ISO 8601 weekly or daily recurrence rule.
    #[serde(default)]
    pub recurrence: Option<String>,
    /// Explicit per-session times, when the cadence isn't easily
    /// described by `recurrence`.
    #[serde(default)]
    pub sessions: Vec<Session>,
}

/// One explicit meeting time within a [`Schedule`], for offerings whose
/// cadence isn't easily described by a `recurrence` rule.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Session {
    /// When the session begins (UTC).
    pub start: DateTime<Utc>,
    /// When the session ends (UTC); `None` if open-ended / unknown.
    #[serde(default)]
    pub end: Option<DateTime<Utc>>,
    /// Optional human-readable label (e.g. "Week 1: Orientation").
    #[serde(default)]
    pub label: Option<String>,
}
