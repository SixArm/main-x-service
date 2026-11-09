//! Row structs: one plain struct per table this service reads whole.
//!
//! Each field is named and typed exactly as its column, so a `query_as!`
//! that selects the columns by name fills the struct, and the compiler
//! checks both against the schema (`.sqlx/`). There is no behaviour here,
//! and no hidden SQL: every statement is written out in the module that
//! runs it (`src/db/mod.rs`, `audit.rs`, `outbox.rs`).
//!
//! Child tables read only in part (identifiers, links, instance
//! languages / instructors / sessions) need no struct: their `query!`
//! records are mapped straight onto the domain types.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A row in the `courses` table; maps to
/// [`Course`](crate::models::Course). The string-list fields live in
/// `course_text_values` and the two credentials in `course_credentials`.
#[derive(Clone, Debug, PartialEq)]
pub struct CourseRow {
    /// Course UUID primary key.
    pub id: Uuid,
    /// Canonical name.
    pub name: String,
    /// SHA-256 (FIPS 180-4) over the assembled record's pre-image.
    ///
    /// `None` on a row written before the column existed — reported
    /// as unhashed, never as a mismatch, and never back-filled.
    pub content_hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image.
    pub content_hash_sha3: Option<String>,
    /// HMAC-SHA256 over the same pre-image — the only one of the
    /// three an adversary holding just this database cannot forge.
    pub content_mac: Option<String>,
    /// Long description.
    pub description: Option<String>,
    /// Disambiguating description.
    pub disambiguating_description: Option<String>,
    /// Canonical URL.
    pub url: Option<String>,
    /// schema.org `additionalType`.
    pub additional_type: Option<String>,
    /// Intended audience.
    pub audience: Option<String>,
    /// License URL.
    pub license: Option<String>,
    /// Typical age range.
    pub typical_age_range: Option<String>,
    /// ISO 8601 duration.
    pub time_required: Option<String>,
    /// Version string.
    pub version: Option<String>,
    /// Free-access flag.
    pub is_accessible_for_free: Option<bool>,
    /// Educational level, stored as its serde string.
    pub educational_level: Option<String>,
    /// Educational use.
    pub educational_use: Option<String>,
    /// Learning-resource type, stored as its serde string.
    pub learning_resource_type: Option<String>,
    /// Interactivity type, stored as its serde string.
    pub interactivity_type: Option<String>,
    /// Provider-scoped course code.
    pub course_code: Option<String>,
    /// Credit count.
    pub number_of_credits: Option<i32>,
    /// Lifetime enrolment count.
    pub total_historical_enrollment: Option<i64>,
    /// Lifecycle status, stored as its serde string.
    pub status: String,
    /// `false` once soft-deleted.
    pub active: bool,
    /// Issuing provider.
    pub provider_id: Option<Uuid>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Soft-delete tombstone.
    pub deleted_at: Option<DateTime<Utc>>,
}

/// A row in the `course_instances` table; maps to
/// [`CourseInstance`](crate::models::CourseInstance). The schedule is
/// flattened onto the row; its sessions, languages, and instructors live
/// in child tables.
#[derive(Clone, Debug, PartialEq)]
pub struct CourseInstanceRow {
    /// Instance UUID primary key.
    pub id: Uuid,
    /// Parent course.
    pub course_id: Uuid,
    /// Display name.
    pub name: Option<String>,
    /// Delivery mode, stored as its serde string.
    pub course_mode: Option<String>,
    /// Lifecycle status, stored as its serde string.
    pub status: String,
    /// Free-text location.
    pub location: Option<String>,
    /// Place-service location id.
    pub location_id: Option<Uuid>,
    /// Capacity.
    pub maximum_attendee_capacity: Option<i32>,
    /// Enrolled count.
    pub enrolled_count: Option<i32>,
    /// Enrolment window start.
    pub enrollment_opens: Option<DateTime<Utc>>,
    /// Enrolment window end.
    pub enrollment_closes: Option<DateTime<Utc>>,
    /// Schedule start.
    pub schedule_start_date: Option<DateTime<Utc>>,
    /// Schedule end.
    pub schedule_end_date: Option<DateTime<Utc>>,
    /// Schedule IANA time zone.
    pub schedule_time_zone: Option<String>,
    /// Schedule recurrence rule.
    pub schedule_recurrence: Option<String>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Soft-delete tombstone.
    pub deleted_at: Option<DateTime<Utc>>,
}

/// A row in `course_text_values`: one entry of one of the course's
/// string-list fields, tagged by `field`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourseTextValueRow {
    /// Which list (`alternate_name`, `keyword`, …).
    pub field: String,
    /// The entry.
    pub value: String,
}

/// A row in `course_credentials`: the educational or occupational
/// credential a course awards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CourseCredentialRow {
    /// `educational` or `occupational`.
    pub role: String,
    /// Credential name.
    pub name: String,
    /// Credential category, stored as its serde string.
    pub category: Option<String>,
    /// Educational level label.
    pub educational_level: Option<String>,
    /// Recognising body.
    pub recognized_by: Option<String>,
    /// Credential URL.
    pub url: Option<String>,
}

/// A row in `course_match_scores` (the persisted review queue).
#[derive(Clone, Debug, PartialEq)]
pub struct ReviewQueueRow {
    /// Review-row UUID primary key.
    pub id: Uuid,
    /// The canonically lower course id.
    pub course_id: Uuid,
    /// The canonically higher course id.
    pub candidate_id: Uuid,
    /// Composite score.
    pub match_score: f64,
    /// Quality bucket.
    pub match_quality: String,
    /// How the pair was found.
    pub detection_method: String,
    /// Per-component breakdown (JSONB).
    pub score_breakdown: Option<serde_json::Value>,
    /// The matcher's evidence (JSONB); `NULL` on rows older than the column.
    pub evidence: Option<serde_json::Value>,
    /// `Pending` / `Confirmed` / `Rejected` / `AutoMerged`.
    pub status: String,
    /// Deciding operator.
    pub reviewed_by: Option<String>,
    /// When the pair was first queued.
    pub created_at: DateTime<Utc>,
    /// When the pair was decided.
    pub reviewed_at: Option<DateTime<Utc>>,
}

/// A row in `audit_log`; projected to
/// [`AuditEntry`](crate::db::audit::AuditEntry) for the API.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditLogRow {
    /// Audit-row UUID primary key.
    pub id: Uuid,
    /// Affected entity kind.
    pub entity_type: String,
    /// Affected entity id.
    pub entity_id: Uuid,
    /// `CREATE` / `UPDATE` / `DELETE`.
    pub action: String,
    /// Acting user id.
    pub user_id: Option<String>,
    /// Originating IP address.
    pub user_ip_address: Option<String>,
    /// Originating user-agent.
    pub user_agent: Option<String>,
    /// Pre-change snapshot (JSONB).
    pub old_values: Option<serde_json::Value>,
    /// Post-change snapshot (JSONB).
    pub new_values: Option<serde_json::Value>,
    /// When the action occurred.
    pub created_at: DateTime<Utc>,
    /// SHA-256 (FIPS 180-4) over this audit row's pre-image.
    ///
    /// Unkeyed, so anyone holding the database can recompute it — what it
    /// catches is careless or unaware modification. Written
    /// unconditionally, unlike the MAC, which needs a key: with no key
    /// configured these two digests are the row's only integrity.
    pub hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
    /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
    /// against one design family does not transfer.
    pub hash_sha3: Option<String>,
    /// HMAC-SHA256 over this audit row's pre-image.
    ///
    /// Detects a row whose content was altered. It does **not**
    /// detect a row deleted wholesale — that needs the hash chain
    /// this service does not yet have (see `crate::compliance`).
    pub mac: Option<String>,
}

/// One persisted `course_outbox` row: a canonical envelope awaiting relay
/// to the durable bus (Phase 2; `agents/share/event-bus.md` §3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboxRow {
    /// `BIGSERIAL` pk; also the global relay order (`ORDER BY id`).
    pub id: i64,
    /// Envelope id — the consumer dedup key.
    pub event_id: Uuid,
    /// The entity name (`course`).
    pub entity: String,
    /// The record pid — the bus partition key.
    pub entity_pid: Uuid,
    /// The change kind: `created` / `updated` / `deleted` / `merged`.
    pub kind: String,
    /// When the change occurred (stamped at enqueue).
    pub occurred_at: DateTime<Utc>,
    /// The acting user pid, or `None` when unauthenticated.
    pub actor: Option<String>,
    /// The envelope schema version.
    pub schema_version: i32,
    /// The full canonical envelope as JSONB.
    pub payload: serde_json::Value,
    /// `None` until the relay ships the row to the bus.
    pub published_at: Option<DateTime<Utc>>,
}
