//! Row structs: one plain struct per table this service reads whole.
//!
//! These persistence rows are deliberately kept separate from the domain
//! models in [`crate::models`]: the domain types are the public,
//! API-facing shapes, while these mirror the normalized relational schema
//! (a worker's names, identifiers, addresses, contacts, documents,
//! emergency contacts, photos and links live in child tables joined by
//! `worker_id`). There is no behaviour here and no hidden SQL: every
//! statement is written out in the module that runs it
//! (`repositories.rs`, `audit.rs`, `outbox.rs`, `entity_links.rs`,
//! `assessments.rs`, `review_queue.rs`, `erasure.rs`).
//!
//! The worker rows use `chrono`, which is what sqlx maps `DATE` and
//! `TIMESTAMPTZ` to and what the domain uses, so they are filled by
//! `query_as!` directly.
//!
//! Four rows leave this layer and keep the `time` types they had under
//! `SeaORM`: [`audit_log::Model`] (the hash chain binds its timestamp, and
//! the audit endpoints serialize it), [`worker_assessments::Model`] (its
//! digest binds its dates), [`entity_links::Model`] and
//! [`event_outbox::Model`]. Each is filled from
//! a `chrono` twin (`*Raw`, crate-private) that `query_as!` fills, converted
//! at the edge by [`super::convert`], so nothing above `src/db/` changes.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::convert::ts_to_offset;

/// One row of `workers`: the core worker record (scalar fields only;
/// names, identifiers and the rest live in child tables). Soft delete is
/// via `deleted_at` / `deleted_by`.
#[derive(Clone, Debug, PartialEq)]
pub struct WorkerRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Whether the record is active.
    pub active: bool,
    /// Worker type tag (`doctor`, `nurse`, …), if recorded.
    pub worker_type: Option<String>,
    /// Gender code (lowercased FHIR value).
    pub gender: String,
    /// Date of birth, if known.
    pub birth_date: Option<NaiveDate>,
    /// Tax identifier (CPF, SSN, TIN), if recorded.
    pub tax_id: Option<String>,
    /// Whether the worker is deceased.
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
    /// Last-update timestamp (the `update_workers_updated_at` trigger
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

/// One row of `worker_names`; `is_primary` flags the worker's main
/// [`HumanName`](crate::models::worker::HumanName).
#[derive(Clone, Debug, PartialEq)]
pub struct NameRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning worker id.
    pub worker_id: Uuid,
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
    /// Whether this is the worker's primary name.
    pub is_primary: bool,
}

/// One row of `worker_identifiers`.
#[derive(Clone, Debug, PartialEq)]
pub struct IdentifierRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning worker id.
    pub worker_id: Uuid,
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

/// One row of `worker_addresses`.
#[derive(Clone, Debug, PartialEq)]
pub struct AddressRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning worker id.
    pub worker_id: Uuid,
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

/// One row of `worker_contacts` (telecom).
#[derive(Clone, Debug, PartialEq)]
pub struct ContactRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning worker id.
    pub worker_id: Uuid,
    /// Contact system tag (`phone`, `email`, …).
    pub system: String,
    /// Contact value.
    pub value: String,
    /// Contact use tag, if recorded.
    pub use_type: Option<String>,
}

/// One row of `worker_links` (worker-to-worker, within this service).
#[derive(Clone, Debug, PartialEq)]
pub struct LinkRow {
    /// Application-assigned primary key.
    pub id: Uuid,
    /// Owning worker id.
    pub worker_id: Uuid,
    /// The other worker's id.
    pub other_worker_id: Uuid,
    /// Link type tag (`replaces`, `replaced_by`, …).
    pub link_type: String,
}

/// One row of `worker_documents`: an identity document.
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

/// One row of `worker_emergency_contacts` (address flattened; telecom
/// lives in `worker_emergency_contact_telecom`).
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

/// One row of `worker_emergency_contact_telecom`.
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
        /// The entity name (`worker`).
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
/// **outbound** edge this service originates from a worker; for v1 the
/// `same_identity` (worker ↔ person) backbone edge (§9). The write is
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
        /// This service's originating worker record (its `id`).
        pub from_pid: Uuid,
        /// Edge kind token (§9 registry), e.g. `same_identity`.
        pub kind: String,
        /// The far record's `EntityRef` URN, e.g. `person:<uuid>`.
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

/// The `worker_assessments` table: one recorded aptitude / personality /
/// psychometric / selection assessment of a worker. Persistence helpers
/// live in [`crate::db::assessments`].
pub mod worker_assessments {
    use serde::{Deserialize, Serialize};
    use time::{Date, OffsetDateTime};
    use uuid::Uuid;

    /// One recorded assessment row.
    #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
    pub struct Model {
        /// Assessment id; application-assigned.
        pub id: Uuid,
        /// The assessed worker's `id`.
        pub worker_id: Uuid,
        /// Category wire token: `aptitude` | `personality` |
        /// `psychometric` | `selection`.
        pub category: String,
        /// The instrument's name.
        pub instrument: String,
        /// The test publisher / administering provider.
        pub provider: Option<String>,
        /// Lifecycle wire token: `scheduled` | `in_progress` |
        /// `completed` | `expired` | `cancelled`.
        pub status: String,
        /// The date the assessment was taken.
        pub administered_on: Option<Date>,
        /// The date the results stop counting as current.
        pub expires_on: Option<Date>,
        /// Who administered it.
        pub administered_by: Option<String>,
        /// Operator notes (sensitive — redacted under `mask`).
        pub notes: Option<String>,
        /// The per-scale outcomes, as a JSON array of result objects.
        pub results: serde_json::Value,
        /// Creation timestamp.
        pub created_at: OffsetDateTime,
        /// Last-update timestamp.
        pub updated_at: OffsetDateTime,
        /// Soft-delete timestamp; `None` while live.
        pub deleted_at: Option<OffsetDateTime>,
        /// SHA-256 over this assessment row
        /// (`crate::compliance::record_integrity`). `None` on rows written
        /// before the column existed; never back-filled, because a
        /// back-fill would certify whatever the current content happens to
        /// be — the claim the hash exists to test.
        pub content_hash: Option<String>,
        /// SHA-3 digest over the same pre-image as `content_hash`.
        pub content_hash_sha3: Option<String>,
        /// HMAC over the same pre-image, as `"<key id>:<hex>"`. Unlike
        /// the digests, not recomputable from the database alone.
        pub content_mac: Option<String>,
    }
}

/// The `chrono` twin of [`worker_assessments::Model`] that `query_as!` fills.
#[derive(Debug)]
pub(crate) struct AssessmentRaw {
    pub id: Uuid,
    pub worker_id: Uuid,
    pub category: String,
    pub instrument: String,
    pub provider: Option<String>,
    pub status: String,
    pub administered_on: Option<NaiveDate>,
    pub expires_on: Option<NaiveDate>,
    pub administered_by: Option<String>,
    pub notes: Option<String>,
    pub results: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub content_hash: Option<String>,
    pub content_hash_sha3: Option<String>,
    pub content_mac: Option<String>,
}

impl From<AssessmentRaw> for worker_assessments::Model {
    fn from(r: AssessmentRaw) -> Self {
        Self {
            id: r.id,
            worker_id: r.worker_id,
            category: r.category,
            instrument: r.instrument,
            provider: r.provider,
            status: r.status,
            administered_on: r.administered_on.map(super::convert::date_to_time),
            expires_on: r.expires_on.map(super::convert::date_to_time),
            administered_by: r.administered_by,
            notes: r.notes,
            results: r.results,
            created_at: ts_to_offset(r.created_at),
            updated_at: ts_to_offset(r.updated_at),
            deleted_at: r.deleted_at.map(ts_to_offset),
            content_hash: r.content_hash,
            content_hash_sha3: r.content_hash_sha3,
            content_mac: r.content_mac,
        }
    }
}
