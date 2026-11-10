//! Row structs: one plain struct per table this service reads whole.
//!
//! These persistence rows are deliberately kept separate from the domain
//! models in [`crate::models`]: the domain types are the public,
//! API-facing shapes, while these mirror the normalized relational schema
//! (a person's names, identifiers, addresses, contacts, documents,
//! emergency contacts, photos and links live in child tables joined by
//! `person_id`). There is no behaviour here and no hidden SQL: every
//! statement is written out in the module that runs it
//! (`repositories.rs`, `audit.rs`, `outbox.rs`, `entity_links.rs`,
//! `bulk_jobs.rs`, `review_queue.rs`, `erasure.rs`).
//!
//! The person rows use `chrono`, which is what sqlx maps `DATE` and
//! `TIMESTAMPTZ` to and what the domain uses, so they are filled by
//! `query_as!` directly.
//!
//! Four rows leave this layer as `time` types, which the code above
//! `src/db/` is written against: [`audit_log::Model`] (the hash chain binds its timestamp, and
//! the audit endpoints serialize it), [`bulk_jobs::Model`],
//! [`entity_links::Model`] and [`event_outbox::Model`]. Each is filled from
//! a `chrono` twin (`*Raw`, crate-private) that `query_as!` fills, converted
//! at the edge by [`super::convert`], so nothing above `src/db/` changes.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::convert::ts_to_offset;

/// One row of `persons`: the core person record (scalar fields only;
/// names, identifiers and the rest live in child tables). Soft delete is
/// via `deleted_at` / `deleted_by`.
#[derive(Clone, Debug, PartialEq)]
pub struct PersonRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Whether the record is active.
    pub active: bool,
    /// Gender code (lowercased FHIR value).
    pub gender: String,
    /// Date of birth, if known.
    pub birth_date: Option<NaiveDate>,
    /// Tax identifier (CPF, SSN, TIN), if recorded.
    pub tax_id: Option<String>,
    /// Whether the person is deceased.
    pub deceased: bool,
    /// Date/time of death, if known.
    pub deceased_datetime: Option<DateTime<Utc>>,
    /// Marital-status code, if recorded.
    pub marital_status: Option<String>,
    /// Multiple-birth indicator, if recorded.
    pub multiple_birth: Option<bool>,
    /// Managing organization id (nullable).
    pub managing_organization_id: Option<Uuid>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last-update timestamp (the `update_patients_updated_at` trigger
    /// stamps it on every `UPDATE`).
    pub updated_at: DateTime<Utc>,
    /// User who created the record.
    pub created_by: Option<String>,
    /// User who last updated the record.
    pub updated_by: Option<String>,
    /// Soft-delete timestamp (set when deleted).
    pub deleted_at: Option<DateTime<Utc>>,
    /// User who soft-deleted the record.
    pub deleted_by: Option<String>,
    /// SHA-256 over the assembled record, for out-of-band tamper
    /// detection (`crate::compliance::record_integrity`). `None` on rows
    /// written before the column existed; never back-filled, because a
    /// back-fill would certify whatever the current content happens to be
    /// — which is the claim the hash exists to test.
    pub content_hash: Option<String>,
    /// SHA-3 digest over the same pre-image as `content_hash`.
    pub content_hash_sha3: Option<String>,
    /// HMAC over the same pre-image, as `"<key id>:<hex>"`. Unlike the
    /// digests, not recomputable from the database alone.
    pub content_mac: Option<String>,
}

/// One row of `person_names`; `is_primary` flags the person's main
/// [`HumanName`](crate::models::person::HumanName).
#[derive(Clone, Debug, PartialEq)]
pub struct NameRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning person id.
    pub person_id: Uuid,
    /// Name use/type code, if recorded.
    pub use_type: Option<String>,
    /// Family (last) name.
    pub family: String,
    /// Given (first/middle) names.
    pub given: Vec<String>,
    /// Name prefixes (e.g. Dr., Mr.).
    pub prefix: Vec<String>,
    /// Name suffixes (e.g. Jr., III).
    pub suffix: Vec<String>,
    /// Whether this is the person's primary name.
    pub is_primary: bool,
}

/// One row of `person_identifiers`.
#[derive(Clone, Debug, PartialEq)]
pub struct IdentifierRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning person id.
    pub person_id: Uuid,
    /// Identifier use code, if recorded.
    pub use_type: Option<String>,
    /// Identifier type tag (`MRN`, `SSN`, …).
    pub identifier_type: String,
    /// Issuing system URI.
    pub system: String,
    /// The identifier value.
    pub value: String,
    /// Assigning authority, if recorded.
    pub assigner: Option<String>,
}

/// One row of `person_addresses`.
#[derive(Clone, Debug, PartialEq)]
pub struct AddressRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning person id.
    pub person_id: Uuid,
    /// Address line 1.
    pub line1: Option<String>,
    /// Address line 2.
    pub line2: Option<String>,
    /// City.
    pub city: Option<String>,
    /// State/region.
    pub state: Option<String>,
    /// Postal code.
    pub postal_code: Option<String>,
    /// Country.
    pub country: Option<String>,
}

/// One row of `person_contacts` (telecom).
#[derive(Clone, Debug, PartialEq)]
pub struct ContactRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning person id.
    pub person_id: Uuid,
    /// Contact system tag (`phone`, `email`, …).
    pub system: String,
    /// Contact value.
    pub value: String,
    /// Contact use tag, if recorded.
    pub use_type: Option<String>,
}

/// One row of `person_links` (person-to-person, within this service).
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning person id.
    pub person_id: Uuid,
    /// The other person's id.
    pub other_person_id: Uuid,
    /// Link type tag (`replaces`, `replaced_by`, …).
    pub link_type: String,
}

/// One row of `person_documents`: an identity document.
#[derive(Clone, Debug, PartialEq)]
pub struct DocumentRow {
    /// Document type tag (e.g. `PASSPORT`).
    pub document_type: String,
    /// Document number.
    pub number: String,
    /// Issuing country, if recorded.
    pub issuing_country: Option<String>,
    /// Issuing authority, if recorded.
    pub issuing_authority: Option<String>,
    /// Issue date, if recorded.
    pub issue_date: Option<NaiveDate>,
    /// Expiry date, if recorded.
    pub expiry_date: Option<NaiveDate>,
    /// Whether the document has been verified.
    pub verified: bool,
}

/// One row of `person_emergency_contacts` (address flattened; telecom
/// lives in `person_emergency_contact_telecom`).
#[derive(Clone, Debug, PartialEq)]
pub struct EmergencyContactRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Contact name.
    pub name: String,
    /// Relationship (spouse, parent, …).
    pub relationship: String,
    /// Whether this is the primary emergency contact.
    pub is_primary: bool,
    /// Flattened address use type.
    pub address_use_type: Option<String>,
    /// Flattened address line 1.
    pub address_line1: Option<String>,
    /// Flattened address line 2.
    pub address_line2: Option<String>,
    /// Flattened address city.
    pub address_city: Option<String>,
    /// Flattened address state.
    pub address_state: Option<String>,
    /// Flattened address postal code.
    pub address_postal_code: Option<String>,
    /// Flattened address country.
    pub address_country: Option<String>,
}

/// One row of `person_emergency_contact_telecom`.
#[derive(Clone, Debug, PartialEq)]
pub struct EmergencyTelecomRow {
    /// Contact system tag.
    pub system: String,
    /// Contact value.
    pub value: String,
    /// Contact use tag, if recorded.
    pub use_type: Option<String>,
}

/// The `audit_log` table: the HIPAA-style change trail
/// (see [`AuditLogRepository`](super::audit::AuditLogRepository)).
pub mod audit_log {
    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;
    use uuid::Uuid;

    /// One audit row: action + entity + old/new JSON + request context.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Model {
        /// Application-assigned primary key.
        pub id: Uuid,
        /// When the audited action occurred.
        pub timestamp: OffsetDateTime,
        /// Acting user id, if known.
        pub user_id: Option<String>,
        /// Action performed (create, update, delete, …).
        pub action: String,
        /// Entity type the action targeted.
        pub entity_type: String,
        /// Id of the affected entity.
        pub entity_id: Uuid,
        /// Pre-change values as JSON, if applicable.
        pub old_values: Option<serde_json::Value>,
        /// Post-change values as JSON, if applicable.
        pub new_values: Option<serde_json::Value>,
        /// Request IP address, if captured.
        pub ip_address: Option<String>,
        /// Request user-agent string, if captured.
        pub user_agent: Option<String>,
        /// Monotonic append order. The primary key is an app-assigned
        /// UUID, which gives no insertion order, and a hash chain needs a
        /// total order — so a `BIGSERIAL` supplies it.
        pub seq: i64,
        /// Hash of the preceding chain row; `None` for the genesis row and
        /// for rows written before the chain existed.
        pub prev_hash: Option<String>,
        /// SHA-3 digest of the preceding chain row.
        pub prev_hash_sha3: Option<String>,
        /// This row's content hash — the link every successor binds to.
        pub hash: Option<String>,
        /// This row's SHA-3 digest — the third parallel chain's link.
        pub hash_sha3: Option<String>,
        /// HMAC over this row's pre-image, as `"<key id>:<hex>"`.
        pub mac: Option<String>,
        /// Request/processing context (purpose-of-use, disclosure recipient).
        pub context: Option<serde_json::Value>,
        /// Whether this access was an outward **disclosure** rather than an
        /// internal access — the HIPAA §164.528 accounting distinction.
        pub disclosure: bool,
        /// When the row's content was destroyed under GDPR Art. 17.
        pub redacted_at: Option<OffsetDateTime>,
        /// Salt of `ip_commitment`, removed with the address. A reader has
        /// no use for it, and it would let a commitment be matched to a
        /// guessed address, so it is never serialized.
        #[serde(skip)]
        pub ip_salt: Option<String>,
        /// What the hash binds in place of the address, which makes this
        /// row chain format `p2`. Kept for good.
        #[serde(skip)]
        pub ip_commitment: Option<String>,
    }
}

/// The `chrono` twin of [`audit_log::Model`] that `query_as!` fills.
#[derive(Debug)]
pub(crate) struct AuditLogRaw {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub seq: i64,
    pub prev_hash: Option<String>,
    pub prev_hash_sha3: Option<String>,
    pub hash: Option<String>,
    pub hash_sha3: Option<String>,
    pub mac: Option<String>,
    pub context: Option<serde_json::Value>,
    pub disclosure: bool,
    pub redacted_at: Option<DateTime<Utc>>,
    pub ip_salt: Option<String>,
    pub ip_commitment: Option<String>,
}

impl From<AuditLogRaw> for audit_log::Model {
    fn from(r: AuditLogRaw) -> Self {
        Self {
            id: r.id,
            timestamp: ts_to_offset(r.timestamp),
            user_id: r.user_id,
            action: r.action,
            entity_type: r.entity_type,
            entity_id: r.entity_id,
            old_values: r.old_values,
            new_values: r.new_values,
            ip_address: r.ip_address,
            user_agent: r.user_agent,
            seq: r.seq,
            prev_hash: r.prev_hash,
            prev_hash_sha3: r.prev_hash_sha3,
            hash: r.hash,
            hash_sha3: r.hash_sha3,
            mac: r.mac,
            context: r.context,
            disclosure: r.disclosure,
            redacted_at: r.redacted_at.map(ts_to_offset),
            ip_salt: r.ip_salt,
            ip_commitment: r.ip_commitment,
        }
    }
}

/// The `event_outbox` table — the transactional-outbox hand-off buffer
/// for the durable event bus (Phase 2; `agents/share/event-bus.md` §3).
/// One row is written **in the same transaction** as the entity mutation,
/// so a committed change always has its event and vice versa; the relay
/// drains unpublished rows to the bus and stamps `published_at`.
pub mod event_outbox {
    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;
    use uuid::Uuid;

    /// One persisted outbox row: a canonical envelope awaiting relay to
    /// the durable bus.
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Model {
        /// Auto-increment pk; also the global relay order (`ORDER BY id`).
        pub id: i64,
        /// Envelope id — the consumer dedup key.
        pub event_id: Uuid,
        /// The entity name (`person`).
        pub entity: String,
        /// The record pid — the bus partition key.
        pub entity_pid: Uuid,
        /// The change kind: `created` / `updated` / `deleted` / `merged`.
        pub kind: String,
        /// When the change occurred (stamped at enqueue).
        pub occurred_at: OffsetDateTime,
        /// The acting user pid, or `None` when unauthenticated.
        pub actor: Option<String>,
        /// The envelope schema version.
        pub schema_version: i32,
        /// The full canonical envelope as JSONB.
        pub payload: serde_json::Value,
        /// `None` until the relay ships the row to the bus.
        pub published_at: Option<OffsetDateTime>,
    }
}

/// The `chrono` twin of [`event_outbox::Model`] that `query_as!` fills.
#[derive(Debug)]
pub(crate) struct OutboxRaw {
    pub id: i64,
    pub event_id: Uuid,
    pub entity: String,
    pub entity_pid: Uuid,
    pub kind: String,
    pub occurred_at: DateTime<Utc>,
    pub actor: Option<String>,
    pub schema_version: i32,
    pub payload: serde_json::Value,
    pub published_at: Option<DateTime<Utc>>,
}

impl From<OutboxRaw> for event_outbox::Model {
    fn from(r: OutboxRaw) -> Self {
        Self {
            id: r.id,
            event_id: r.event_id,
            entity: r.entity,
            entity_pid: r.entity_pid,
            kind: r.kind,
            occurred_at: ts_to_offset(r.occurred_at),
            actor: r.actor,
            schema_version: r.schema_version,
            payload: r.payload,
            published_at: r.published_at.map(ts_to_offset),
        }
    }
}

/// The `entity_links` table — the cross-service linking **write side**
/// (`agents/share/cross-service-linking.md` §4.1). Each row is one
/// **outbound** edge this service originates from a person; for v1 the
/// `same_identity` (person ↔ worker) backbone edge (§9). The write is
/// optimistic — it stores the assertion and never calls the target
/// service; verification is the read-model aggregator's concern.
/// Persistence helpers (idempotent upsert, list-active, bulk-list,
/// soft-delete) live in [`crate::db::entity_links`]; edge **validation**
/// (parse `to_ref`, resolve the kind, check the endpoint pair) is DB-free
/// in `api::rest::links`.
pub mod entity_links {
    use serde::{Deserialize, Serialize};
    use time::{Date, OffsetDateTime};
    use uuid::Uuid;

    /// One outbound cross-service edge row.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Model {
        /// Edge id (also the event's `edge_id`); application-assigned.
        pub id: Uuid,
        /// This service's originating person record (its `id`).
        pub from_pid: Uuid,
        /// Edge kind token (§9 registry), e.g. `same_identity`.
        pub kind: String,
        /// The far record's `EntityRef` URN, e.g. `worker:<uuid>`.
        pub to_ref: String,
        /// Optional role label (unused by `same_identity`).
        pub role: Option<String>,
        /// Optional confidence: `1.0` operator-asserted, `<1` suggested.
        pub confidence: Option<f64>,
        /// How the edge arose: `operator` | `import` | `matcher_suggested`.
        pub provenance: String,
        /// Affiliation start (nullable — `same_identity` is not temporal).
        pub valid_from: Option<Date>,
        /// Affiliation end ("former …" once past).
        pub valid_to: Option<Date>,
        /// Creation timestamp.
        pub created_at: OffsetDateTime,
        /// Soft-delete timestamp (a withdrawn edge); `None` while live.
        pub deleted_at: Option<OffsetDateTime>,
    }
}

/// The `chrono` twin of [`entity_links::Model`] that `query_as!` fills.
#[derive(Debug)]
pub(crate) struct EntityLinkRaw {
    pub id: Uuid,
    pub from_pid: Uuid,
    pub kind: String,
    pub to_ref: String,
    pub role: Option<String>,
    pub confidence: Option<f64>,
    pub provenance: String,
    pub valid_from: Option<NaiveDate>,
    pub valid_to: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl From<EntityLinkRaw> for entity_links::Model {
    fn from(r: EntityLinkRaw) -> Self {
        Self {
            id: r.id,
            from_pid: r.from_pid,
            kind: r.kind,
            to_ref: r.to_ref,
            role: r.role,
            confidence: r.confidence,
            provenance: r.provenance,
            valid_from: r.valid_from.map(super::convert::date_to_time),
            valid_to: r.valid_to.map(super::convert::date_to_time),
            created_at: ts_to_offset(r.created_at),
            deleted_at: r.deleted_at.map(ts_to_offset),
        }
    }
}

/// The `bulk_jobs` table: one async bulk import/export operation, drained
/// by a queue worker (`agents/share/bulk-import-export.md` §3).
pub mod bulk_jobs {
    use serde::{Deserialize, Serialize};
    use time::OffsetDateTime;
    use uuid::Uuid;

    /// One bulk-job row: kind + format + status, per-row counts, and the
    /// artifact references (input, result, error report).
    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Model {
        /// Application-assigned primary key (the job id).
        pub id: Uuid,
        /// `import` or `export`.
        pub kind: String,
        /// The entity name (`person`).
        pub entity: String,
        /// File format token (`jsonl`).
        pub format: String,
        /// `queued|running|completed|completed_with_errors|failed`.
        pub status: String,
        /// Free-form job parameters (dry-run flag, export filter, …).
        pub params: serde_json::Value,
        /// Total record rows seen, once known.
        pub rows_total: Option<i64>,
        /// Rows processed so far.
        pub rows_processed: i64,
        /// Rows inserted as new records.
        pub rows_created: i64,
        /// Rows upserted onto an existing record (idempotent re-import).
        pub rows_upserted: i64,
        /// Rows routed to the duplicate review queue (deferred; always 0).
        pub rows_to_review: i64,
        /// Rows that failed validation/parse/persist.
        pub rows_errored: i64,
        /// Acting user pid (bearer `sub`), if any.
        pub actor: Option<String>,
        /// Client-supplied idempotency key, if any.
        pub idempotency_key: Option<String>,
        /// Artifact reference for the uploaded source file.
        pub input_url: Option<String>,
        /// Artifact reference for the export output / import receipt.
        pub result_url: Option<String>,
        /// Artifact reference for the downloadable per-row error report.
        pub error_report_url: Option<String>,
        /// Creation timestamp.
        pub created_at: OffsetDateTime,
        /// Last-update timestamp.
        pub updated_at: OffsetDateTime,
        /// Artifact + row TTL, if set.
        pub expires_at: Option<OffsetDateTime>,
        /// When the periodic sweep (SEC-B4 follow-up, `crate::bulk::sweep`)
        /// physically deleted this job's artifacts from the store. `None`
        /// means either not yet expired, or expired but not yet swept.
        pub artifact_deleted_at: Option<OffsetDateTime>,
    }
}

/// The `chrono` twin of [`bulk_jobs::Model`] that `query_as!` fills.
#[derive(Debug)]
pub(crate) struct BulkJobRaw {
    pub id: Uuid,
    pub kind: String,
    pub entity: String,
    pub format: String,
    pub status: String,
    pub params: serde_json::Value,
    pub rows_total: Option<i64>,
    pub rows_processed: i64,
    pub rows_created: i64,
    pub rows_upserted: i64,
    pub rows_to_review: i64,
    pub rows_errored: i64,
    pub actor: Option<String>,
    pub idempotency_key: Option<String>,
    pub input_url: Option<String>,
    pub result_url: Option<String>,
    pub error_report_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub artifact_deleted_at: Option<DateTime<Utc>>,
}

impl From<BulkJobRaw> for bulk_jobs::Model {
    fn from(r: BulkJobRaw) -> Self {
        Self {
            id: r.id,
            kind: r.kind,
            entity: r.entity,
            format: r.format,
            status: r.status,
            params: r.params,
            rows_total: r.rows_total,
            rows_processed: r.rows_processed,
            rows_created: r.rows_created,
            rows_upserted: r.rows_upserted,
            rows_to_review: r.rows_to_review,
            rows_errored: r.rows_errored,
            actor: r.actor,
            idempotency_key: r.idempotency_key,
            input_url: r.input_url,
            result_url: r.result_url,
            error_report_url: r.error_report_url,
            created_at: ts_to_offset(r.created_at),
            updated_at: ts_to_offset(r.updated_at),
            expires_at: r.expires_at.map(ts_to_offset),
            artifact_deleted_at: r.artifact_deleted_at.map(ts_to_offset),
        }
    }
}
