//! `CourseInstance` — schema.org/CourseInstance.
//!
//! A specific offering of a `Course` at a particular time / place /
//! mode (e.g. CS101 Fall 2026 with Prof. Smith). Multiple instances
//! can share the same parent `Course`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseInstance {
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

    /// Enrollment opens / closes — ISO 8601 in UTC.
    #[serde(default)]
    pub enrollment_opens: Option<DateTime<Utc>>,
    #[serde(default)]
    pub enrollment_closes: Option<DateTime<Utc>>,

    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

/// schema.org/CourseInstance.courseMode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseMode {
    Online,
    Onsite,
    Blended,
    SelfPaced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CourseInstanceStatus {
    #[default]
    Scheduled,
    EnrollmentOpen,
    EnrollmentClosed,
    InProgress,
    Completed,
    Cancelled,
}

/// Time window for a course instance. Either a single
/// `start_date`/`end_date` window or an explicit list of session
/// times; both are optional so this also accommodates self-paced
/// offerings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    #[serde(default)]
    pub start_date: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub start: DateTime<Utc>,
    #[serde(default)]
    pub end: Option<DateTime<Utc>>,
    #[serde(default)]
    pub label: Option<String>,
}
