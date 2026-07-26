//! Repository layer translating between domain [`Worker`]s and SeaORM rows.
//!
//! [`WorkerRepository`] is the async trait the API depends on;
//! [`SeaOrmWorkerRepository`] is the PostgreSQL/SeaORM implementation. The
//! implementation owns the mapping in both directions (`to_active_models` /
//! `from_db_models`), runs multi-table writes in a transaction, and — when
//! configured — publishes [`crate::streaming::WorkerEvent`]s and writes audit
//! entries on every mutation. [`AuditContext`] carries the actor metadata
//! recorded alongside those audit entries.

use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use time::OffsetDateTime;
use uuid::Uuid;

use super::convert::{date_to_time, offset_to_ts, time_to_date, ts_to_offset};

use super::models::{
    worker_addresses, worker_contacts, worker_documents, worker_emergency_contact_telecom,
    worker_emergency_contacts, worker_identifiers, worker_links, worker_names, worker_photos,
    workers,
};
use crate::Result;
use crate::db::outbox::OutboxInsert;
use crate::models::{
    Address, ContactPoint, ContactPointSystem, DocumentType, EmergencyContact, HumanName,
    Identifier, IdentityDocument, Worker, WorkerLink, WorkerType,
};
use crate::streaming::{EventKind, EventTransport};

/// The full set of `SeaORM` `ActiveModel`s produced from one [`Worker`]: the
/// core row plus its name / identifier / address / contact / link child rows.
type WorkerActiveModels = (
    workers::ActiveModel,
    Vec<worker_names::ActiveModel>,
    Vec<worker_identifiers::ActiveModel>,
    Vec<worker_addresses::ActiveModel>,
    Vec<worker_contacts::ActiveModel>,
    Vec<worker_links::ActiveModel>,
);

/// Serialize a fieldless enum to its canonical serde string tag.
///
/// Used to store domain enums (document type, contact-point system, etc.) as
/// their externally-tagged string form in a single text column. Returns `None`
/// if the value does not serialize to a bare JSON string (i.e. it is not a
/// fieldless/unit enum variant).
fn enum_to_tag<T: serde::Serialize>(value: &T) -> Option<String> {
    // Round-trip through serde_json so the stored tag matches the enum's
    // `#[serde(rename = ...)]` representation exactly.
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
}

/// Parse a stored string tag back into a fieldless enum, or `None`.
///
/// Inverse of [`enum_to_tag`]: wraps the stored tag as a JSON string and
/// deserializes it. Returns `None` for a missing tag or one that no longer
/// maps to a known variant.
fn tag_to_enum<T: serde::de::DeserializeOwned>(tag: Option<&str>) -> Option<T> {
    let t = tag?;
    serde_json::from_value(serde_json::Value::String(t.to_string())).ok()
}

/// Insert the normalized document / emergency-contact (+ telecom) / photo
/// child rows for `worker` on connection `conn`.
///
/// These three collections live in their own tables (rather than the flat
/// name/identifier/address/contact set handled by `to_active_models`) and
/// preserve list order via a `position` column. Each child row gets a fresh
/// UUID; emergency-contact telecom rows reference the parent contact's UUID.
/// Generic over `ConnectionTrait` so it runs equally on a pooled connection or
/// inside a transaction.
///
/// # Errors
///
/// Returns an error if any child-row insert fails; when run inside a
/// transaction the caller rolls the whole write back.
async fn insert_extra_collections<C: sea_orm::ConnectionTrait>(
    conn: &C,
    worker: &Worker,
) -> Result<()> {
    for (i, doc) in worker.documents.iter().enumerate() {
        worker_documents::ActiveModel {
            id: Set(Uuid::new_v4()),
            worker_id: Set(worker.id),
            document_type: Set(enum_to_tag(&doc.document_type).unwrap_or_default()),
            number: Set(doc.number.clone()),
            issuing_country: Set(doc.issuing_country.clone()),
            issuing_authority: Set(doc.issuing_authority.clone()),
            issue_date: Set(doc.issue_date.map(date_to_time)),
            expiry_date: Set(doc.expiry_date.map(date_to_time)),
            verified: Set(doc.verified),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await?;
    }
    for (i, ec) in worker.emergency_contacts.iter().enumerate() {
        let ec_id = Uuid::new_v4();
        let addr = ec.address.as_ref();
        worker_emergency_contacts::ActiveModel {
            id: Set(ec_id),
            worker_id: Set(worker.id),
            name: Set(ec.name.clone()),
            relationship: Set(ec.relationship.clone()),
            is_primary: Set(ec.is_primary),
            address_use_type: Set(addr.and_then(|a| a.use_type.as_ref().and_then(enum_to_tag))),
            address_line1: Set(addr.and_then(|a| a.line1.clone())),
            address_line2: Set(addr.and_then(|a| a.line2.clone())),
            address_city: Set(addr.and_then(|a| a.city.clone())),
            address_state: Set(addr.and_then(|a| a.state.clone())),
            address_postal_code: Set(addr.and_then(|a| a.postal_code.clone())),
            address_country: Set(addr.and_then(|a| a.country.clone())),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await?;
        for (j, cp) in ec.telecom.iter().enumerate() {
            worker_emergency_contact_telecom::ActiveModel {
                id: Set(Uuid::new_v4()),
                emergency_contact_id: Set(ec_id),
                system: Set(enum_to_tag(&cp.system).unwrap_or_default()),
                value: Set(cp.value.clone()),
                use_type: Set(cp.use_type.as_ref().and_then(enum_to_tag)),
                position: Set(i32::try_from(j).unwrap_or(i32::MAX)),
            }
            .insert(conn)
            .await?;
        }
    }
    for (i, url) in worker.photo.iter().enumerate() {
        worker_photos::ActiveModel {
            id: Set(Uuid::new_v4()),
            worker_id: Set(worker.id),
            url: Set(url.clone()),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await?;
    }
    Ok(())
}

/// The content hash a record should carry after an in-place update.
///
/// Reads the row's existing soft-delete stamp, which the digest binds and
/// which an `..Default::default()` update leaves untouched, so it cannot
/// come from the domain model (which does not carry it).
///
/// Extracted because both `apply_worker_row_replacement` and the `update`
/// implementation need it — this crate writes the parent row in two
/// places, pre-existing drift that is at least not duplicated further
/// here.
///
/// Note the absence of compiler help at either call site: both build their
/// `ActiveModel` with `..Default::default()`, so adding `content_hash` to
/// the entity did not break them the way it broke the create path. A write
/// that forgets to rehash produces a *false* tamper report, which is worse
/// than no control at all; the DB-gated tests exercise every path for
/// exactly this reason.
///
/// # Errors
///
/// Returns an error if the lookup fails or the record cannot be hashed.
async fn content_hash_for_update<C: ConnectionTrait>(conn: &C, worker: &Worker) -> Result<String> {
    let existing_deleted_at = workers::Entity::find_by_id(worker.id)
        .one(conn)
        .await?
        .and_then(|row| row.deleted_at)
        .and_then(|d| i64::try_from(d.unix_timestamp_nanos() / 1_000).ok());
    crate::compliance::record_integrity::hash_with_deleted_at(worker, existing_deleted_at)
}

/// Apply a worker's parent-row update and wholesale child-row replacement on
/// `conn`: update the `workers` row (stamping `updated_at`), delete every
/// child row for the worker, then re-insert them from the domain model.
///
/// Runs on the caller's connection/transaction so it shares that commit
/// boundary. Used by [`WorkerRepository::merge`] to apply the survivor's
/// rows atomically alongside the duplicate's soft-delete and the outbox rows.
///
/// # Errors
///
/// Returns an error if any update/delete/insert query fails.
async fn apply_worker_row_replacement<C: ConnectionTrait>(conn: &C, worker: &Worker) -> Result<()> {
    let content_hash = content_hash_for_update(conn, worker).await?;

    let update_model = workers::ActiveModel {
        id: Set(worker.id),
        active: Set(worker.active),
        content_hash: Set(Some(content_hash)),
        worker_type: Set(worker
            .worker_type
            .as_ref()
            .map(std::string::ToString::to_string)),
        // Lowercased to honor the `workers_gender_check` CHECK constraint
        // (`'male'`/`'female'`/`'other'`/`'unknown'`); the bare `Debug`
        // form is rejected by the database.
        gender: Set(format!("{:?}", worker.gender).to_lowercase()),
        birth_date: Set(worker.birth_date.map(date_to_time)),
        tax_id: Set(worker.tax_id.clone()),
        deceased: Set(worker.deceased),
        deceased_datetime: Set(worker.deceased_datetime.map(ts_to_offset)),
        marital_status: Set(worker.marital_status.clone()),
        multiple_birth: Set(worker.multiple_birth),
        managing_organization_id: Set(worker.managing_organization),
        updated_at: Set(OffsetDateTime::now_utc()),
        updated_by: Set(None),
        ..Default::default()
    };
    update_model.update(conn).await?;

    worker_names::Entity::delete_many()
        .filter(worker_names::Column::WorkerId.eq(worker.id))
        .exec(conn)
        .await?;
    worker_identifiers::Entity::delete_many()
        .filter(worker_identifiers::Column::WorkerId.eq(worker.id))
        .exec(conn)
        .await?;
    worker_addresses::Entity::delete_many()
        .filter(worker_addresses::Column::WorkerId.eq(worker.id))
        .exec(conn)
        .await?;
    worker_contacts::Entity::delete_many()
        .filter(worker_contacts::Column::WorkerId.eq(worker.id))
        .exec(conn)
        .await?;
    worker_links::Entity::delete_many()
        .filter(worker_links::Column::WorkerId.eq(worker.id))
        .exec(conn)
        .await?;
    worker_emergency_contacts::Entity::delete_many()
        .filter(worker_emergency_contacts::Column::WorkerId.eq(worker.id))
        .exec(conn)
        .await?;
    worker_documents::Entity::delete_many()
        .filter(worker_documents::Column::WorkerId.eq(worker.id))
        .exec(conn)
        .await?;
    worker_photos::Entity::delete_many()
        .filter(worker_photos::Column::WorkerId.eq(worker.id))
        .exec(conn)
        .await?;

    let (_, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
        SeaOrmWorkerRepository::to_active_models(worker)?;
    for name in new_names {
        name.insert(conn).await?;
    }
    for identifier in new_identifiers {
        identifier.insert(conn).await?;
    }
    for address in new_addresses {
        address.insert(conn).await?;
    }
    for contact in new_contacts {
        contact.insert(conn).await?;
    }
    for link in new_links {
        link.insert(conn).await?;
    }
    insert_extra_collections(conn, worker).await?;
    Ok(())
}

/// Actor metadata attached to audit-log entries for a mutation.
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// Acting user's identifier (defaults to `"system"`).
    pub user_id: Option<String>,
    /// Originating client IP address, if known.
    pub ip_address: Option<String>,
    /// Originating client user-agent string, if known.
    pub user_agent: Option<String>,
}

impl Default for AuditContext {
    /// Returns a context attributed to the `"system"` user with no IP or
    /// user-agent — used for internal/automated operations.
    fn default() -> Self {
        Self {
            user_id: Some("system".to_string()),
            ip_address: None,
            user_agent: None,
        }
    }
}

/// Persistence operations for worker records. Implementors must be `Send +
/// Sync` so the repository can be shared across async tasks behind an `Arc`.
#[async_trait::async_trait]
pub trait WorkerRepository: Send + Sync {
    /// Persists a new worker (and its names/identifiers/addresses/contacts/
    /// links), returning the stored record.
    ///
    /// # Errors
    ///
    /// Returns an error if the transactional insert fails, or if reloading the
    /// just-written record fails (e.g. a missing primary name).
    async fn create(&self, worker: &Worker) -> Result<Worker>;

    /// Fetches a non-deleted worker by ID, or `None` if absent/soft-deleted.
    ///
    /// # Errors
    ///
    /// Returns an error if a database query fails, or if a found row cannot be
    /// reassembled into a domain [`Worker`].
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Worker>>;

    /// Replaces a worker and its associated rows, returning the new state.
    ///
    /// # Errors
    ///
    /// Returns an error if the transactional update fails, or if the worker is
    /// not found after the update.
    async fn update(&self, worker: &Worker) -> Result<Worker>;

    /// Merges the `duplicate_id` record into `survivor`: applies the
    /// survivor's parent + child row updates and soft-deletes the
    /// duplicate **in one transaction**, so the whole merge commits (or
    /// rolls back) atomically. Under the outbox transport this enqueues a
    /// `Merged` outbox row for the survivor (carrying the duplicate's pid
    /// via `merged_from`) and a `Deleted` outbox row for the duplicate on
    /// the same transaction. Returns the reloaded survivor.
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction or any query fails, or
    /// [`crate::Error::Validation`] if the survivor cannot be re-read after
    /// the merge.
    async fn merge(&self, survivor: &Worker, duplicate_id: &Uuid) -> Result<Worker>;

    /// Soft-deletes a worker by stamping its `deleted_at` column.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// Finds workers whose family name matches `query` (case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns an error if a database query fails.
    async fn search(&self, query: &str) -> Result<Vec<Worker>>;

    /// Returns a page of active, non-deleted workers.
    ///
    /// # Errors
    ///
    /// Returns an error if a database query fails.
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Worker>>;
}

/// SeaORM/PostgreSQL implementation of [`WorkerRepository`].
///
/// Optionally wired with an event publisher and an audit-log repository via
/// the `with_*` builder methods; when present, every mutation publishes an
/// event and records an audit entry.
pub struct SeaOrmWorkerRepository {
    /// Pooled database connection.
    db: DatabaseConnection,
    /// Optional event publisher; mutations emit events when set.
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    /// Optional audit repository; mutations log entries when set.
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps the legacy post-commit
    /// ring-buffer publish; [`EventTransport::Outbox`] additionally writes
    /// one `event_outbox` row **inside** each write's transaction.
    transport: EventTransport,
}

impl SeaOrmWorkerRepository {
    /// Builds a repository over `db` with no event publisher or audit log,
    /// and the default [`EventTransport::Memory`] transport (behaviour
    /// unchanged from before the outbox landed).
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            event_publisher: None,
            audit_log: None,
            transport: EventTransport::Memory,
        }
    }

    /// Builder: select the event transport (see [`EventTransport`]).
    /// `AppState` wires this from `WORKER_EVENT_TRANSPORT` via
    /// [`crate::streaming::transport`].
    #[must_use]
    pub fn with_transport(mut self, transport: EventTransport) -> Self {
        self.transport = transport;
        self
    }

    /// When the outbox transport is active, enqueue one `event_outbox`
    /// row for `kind` applied to `worker` **on `conn`** — pass the open
    /// `&DatabaseTransaction` so the row commits with the entity write
    /// (the outbox atomicity guarantee). A no-op under
    /// [`EventTransport::Memory`].
    ///
    /// The `ConnectionTrait` generic lives here on a concrete method (not
    /// on the object-safe [`WorkerRepository`] trait), which is how a
    /// `dyn`-trait repository threads the outbox insert into its own
    /// transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if the outbox `INSERT` fails (e.g. a duplicate
    /// `event_id`) or the envelope cannot be built.
    async fn enqueue_outbox<C: ConnectionTrait>(
        &self,
        conn: &C,
        worker: &Worker,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_event(worker, kind)?
                .insert_on(conn)
                .await?;
        }
        Ok(())
    }

    /// Builder method attaching an event publisher; returns `self` for chaining.
    #[must_use]
    pub fn with_event_publisher(
        mut self,
        publisher: std::sync::Arc<dyn crate::streaming::EventProducer>,
    ) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Builder method attaching an audit-log repository; returns `self` for chaining.
    #[must_use]
    pub fn with_audit_log(
        mut self,
        audit_log: std::sync::Arc<super::audit::AuditLogRepository>,
    ) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// Publishes `event` if a publisher is configured; logs (but swallows) any
    /// publish error so a streaming failure never aborts the database write.
    fn publish_event(&self, event: crate::streaming::WorkerEvent) {
        if let Some(ref publisher) = self.event_publisher
            && let Err(e) = publisher.publish(event)
        {
            tracing::error!("Failed to publish event: {}", e);
        }
    }

    /// Writes a `CREATE`/`UPDATE`/`DELETE` audit entry for a `Worker` if an
    /// audit log is configured. Unknown actions are no-ops; errors are logged
    /// but not propagated.
    ///
    /// Dispatches on `action` to the matching typed helper on the audit
    /// repository, passing the JSON snapshots and actor metadata. A `None`
    /// snapshot is substituted with JSON `null` for the column. Like
    /// [`Self::publish_event`], an audit failure is logged but never surfaced,
    /// so the audit trail is best-effort and decoupled from the primary write.
    async fn log_audit(
        &self,
        action: &str,
        entity_id: uuid::Uuid,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        context: &AuditContext,
    ) {
        if let Some(ref audit_log) = self.audit_log {
            let actor = super::audit::AuditActor {
                user_id: context.user_id.as_deref(),
                ip_address: context.ip_address.as_deref(),
                user_agent: context.user_agent.as_deref(),
            };
            let result = match action {
                "CREATE" => {
                    audit_log
                        .log_create(
                            "Worker",
                            entity_id,
                            new_values.unwrap_or(serde_json::Value::Null),
                            &actor,
                        )
                        .await
                }
                "UPDATE" => {
                    audit_log
                        .log_update(
                            "Worker",
                            entity_id,
                            old_values.unwrap_or(serde_json::Value::Null),
                            new_values.unwrap_or(serde_json::Value::Null),
                            &actor,
                        )
                        .await
                }
                "DELETE" => {
                    audit_log
                        .log_delete(
                            "Worker",
                            entity_id,
                            old_values.unwrap_or(serde_json::Value::Null),
                            &actor,
                        )
                        .await
                }
                _ => Ok(()),
            };

            if let Err(e) = result {
                tracing::error!("Failed to log audit: {}", e);
            }
        }
    }

    /// Builds the `worker_names` rows for `worker`: the primary name (flagged
    /// `is_primary`) followed by any additional names.
    fn name_active_models(worker: &Worker) -> Vec<worker_names::ActiveModel> {
        let mut names = vec![worker_names::ActiveModel {
            id: Set(Uuid::new_v4()),
            worker_id: Set(worker.id),
            use_type: Set(worker.name.use_type.as_ref().map(|u| format!("{u:?}"))),
            family: Set(worker.name.family.clone()),
            given: Set(worker.name.given.clone()),
            prefix: Set(worker.name.prefix.clone()),
            suffix: Set(worker.name.suffix.clone()),
            is_primary: Set(true),
            created_at: Set(OffsetDateTime::now_utc()),
            updated_at: Set(OffsetDateTime::now_utc()),
        }];

        for add_name in &worker.additional_names {
            names.push(worker_names::ActiveModel {
                id: Set(Uuid::new_v4()),
                worker_id: Set(worker.id),
                use_type: Set(add_name.use_type.as_ref().map(|u| format!("{u:?}"))),
                family: Set(add_name.family.clone()),
                given: Set(add_name.given.clone()),
                prefix: Set(add_name.prefix.clone()),
                suffix: Set(add_name.suffix.clone()),
                is_primary: Set(false),
                created_at: Set(OffsetDateTime::now_utc()),
                updated_at: Set(OffsetDateTime::now_utc()),
            });
        }
        names
    }

    /// Flattens a domain [`Worker`] into the set of `SeaORM` `ActiveModel`s for
    /// each backing table (worker row plus name/identifier/address/contact/
    /// link rows). The first name is marked primary; the first address and
    /// contact are marked primary. Demographic/use-type enums are stored via
    /// their `Debug` form (`format!("{:?}", …)`) — except `gender`, which is
    /// lowercased to honor the `workers_gender_check` CHECK constraint —
    /// while `worker_type` uses its `Display`/`to_string`;
    /// [`from_db_models`](Self::from_db_models) parses these string forms
    /// back. Fresh UUIDs and `now_utc()` timestamps are
    /// assigned to child rows. This does not cover the document / emergency-
    /// contact / photo collections — those are handled by
    /// [`insert_extra_collections`].
    fn to_active_models(worker: &Worker) -> Result<WorkerActiveModels> {
        let new_worker = workers::ActiveModel {
            id: Set(worker.id),
            active: Set(worker.active),
            // A new record is live, so the digest binds `deleted_at` as
            // `None`.
            content_hash: Set(Some(crate::compliance::record_integrity::hash_of_live(
                worker,
            )?)),
            worker_type: Set(worker
                .worker_type
                .as_ref()
                .map(std::string::ToString::to_string)),
            // Lowercased for the `workers_gender_check` CHECK constraint.
            gender: Set(format!("{:?}", worker.gender).to_lowercase()),
            birth_date: Set(worker.birth_date.map(date_to_time)),
            tax_id: Set(worker.tax_id.clone()),
            deceased: Set(worker.deceased),
            deceased_datetime: Set(worker.deceased_datetime.map(ts_to_offset)),
            marital_status: Set(worker.marital_status.clone()),
            multiple_birth: Set(worker.multiple_birth),
            managing_organization_id: Set(worker.managing_organization),
            created_at: Set(OffsetDateTime::now_utc()),
            updated_at: Set(OffsetDateTime::now_utc()),
            created_by: Set(None),
            updated_by: Set(None),
            deleted_at: Set(None),
            deleted_by: Set(None),
        };

        let names = Self::name_active_models(worker);

        // Identifiers
        let identifiers = worker
            .identifiers
            .iter()
            .map(|id| worker_identifiers::ActiveModel {
                id: Set(Uuid::new_v4()),
                worker_id: Set(worker.id),
                use_type: Set(id.use_type.as_ref().map(|u| format!("{u:?}"))),
                identifier_type: Set(format!("{:?}", id.identifier_type)),
                system: Set(id.system.clone()),
                value: Set(id.value.clone()),
                assigner: Set(id.assigner.clone()),
                created_at: Set(OffsetDateTime::now_utc()),
                updated_at: Set(OffsetDateTime::now_utc()),
            })
            .collect();

        // Addresses
        let addresses = worker
            .addresses
            .iter()
            .enumerate()
            .map(|(idx, addr)| worker_addresses::ActiveModel {
                id: Set(Uuid::new_v4()),
                worker_id: Set(worker.id),
                use_type: Set(None),
                line1: Set(addr.line1.clone()),
                line2: Set(addr.line2.clone()),
                city: Set(addr.city.clone()),
                state: Set(addr.state.clone()),
                postal_code: Set(addr.postal_code.clone()),
                country: Set(addr.country.clone()),
                is_primary: Set(idx == 0),
                created_at: Set(OffsetDateTime::now_utc()),
                updated_at: Set(OffsetDateTime::now_utc()),
            })
            .collect();

        // Contacts
        let contacts = worker
            .telecom
            .iter()
            .enumerate()
            .map(|(idx, cp)| worker_contacts::ActiveModel {
                id: Set(Uuid::new_v4()),
                worker_id: Set(worker.id),
                system: Set(format!("{:?}", cp.system)),
                value: Set(cp.value.clone()),
                use_type: Set(cp.use_type.as_ref().map(|u| format!("{u:?}"))),
                is_primary: Set(idx == 0),
                created_at: Set(OffsetDateTime::now_utc()),
                updated_at: Set(OffsetDateTime::now_utc()),
            })
            .collect();

        // Links
        let links = worker
            .links
            .iter()
            .map(|link| worker_links::ActiveModel {
                id: Set(Uuid::new_v4()),
                worker_id: Set(worker.id),
                other_worker_id: Set(link.other_worker_id),
                link_type: Set(format!("{:?}", link.link_type)),
                created_at: Set(OffsetDateTime::now_utc()),
                created_by: Set(None),
            })
            .collect();

        Ok((new_worker, names, identifiers, addresses, contacts, links))
    }

    /// Maps one `worker_names` row back into a domain [`HumanName`], parsing
    /// the string-stored name-use tag.
    fn human_name_from_db(n: &worker_names::Model) -> HumanName {
        use crate::models::NameUse;

        HumanName {
            use_type: n.use_type.as_ref().and_then(|u| match u.as_str() {
                "Usual" => Some(NameUse::Usual),
                "Official" => Some(NameUse::Official),
                "Temp" => Some(NameUse::Temp),
                "Nickname" => Some(NameUse::Nickname),
                "Anonymous" => Some(NameUse::Anonymous),
                "Old" => Some(NameUse::Old),
                "Maiden" => Some(NameUse::Maiden),
                _ => None,
            }),
            family: n.family.clone(),
            given: n.given.clone(),
            prefix: n.prefix.clone(),
            suffix: n.suffix.clone(),
        }
    }

    /// Maps the `worker_identifiers` rows back into domain [`Identifier`]s,
    /// parsing the string-stored type / use-type tags.
    fn identifiers_from_db(db_identifiers: &[worker_identifiers::Model]) -> Vec<Identifier> {
        use crate::models::{IdentifierType, IdentifierUse};

        db_identifiers
            .iter()
            .map(|id| {
                let identifier_type = match id.identifier_type.as_str() {
                    "MRN" => IdentifierType::MRN,
                    "SSN" => IdentifierType::SSN,
                    "DL" => IdentifierType::DL,
                    "NPI" => IdentifierType::NPI,
                    "PPN" => IdentifierType::PPN,
                    "TAX" => IdentifierType::TAX,
                    "ODS" => IdentifierType::ODS,
                    _ => IdentifierType::Other,
                };

                let use_type = id.use_type.as_ref().and_then(|u| match u.as_str() {
                    "Usual" => Some(IdentifierUse::Usual),
                    "Official" => Some(IdentifierUse::Official),
                    "Temp" => Some(IdentifierUse::Temp),
                    "Secondary" => Some(IdentifierUse::Secondary),
                    "Old" => Some(IdentifierUse::Old),
                    _ => None,
                });

                Identifier {
                    identifier_type,
                    use_type,
                    system: id.system.clone(),
                    value: id.value.clone(),
                    assigner: id.assigner.clone(),
                }
            })
            .collect()
    }

    /// Maps the `worker_contacts` rows back into domain [`ContactPoint`]s,
    /// dropping any row whose system tag is unrecognized.
    fn telecom_from_db(db_contacts: &[worker_contacts::Model]) -> Vec<ContactPoint> {
        use crate::models::{ContactPointSystem, ContactPointUse};

        db_contacts
            .iter()
            .filter_map(|cp| {
                let system = match cp.system.as_str() {
                    "Phone" => ContactPointSystem::Phone,
                    "Fax" => ContactPointSystem::Fax,
                    "Email" => ContactPointSystem::Email,
                    "Pager" => ContactPointSystem::Pager,
                    "Url" => ContactPointSystem::Url,
                    "Sms" => ContactPointSystem::Sms,
                    "Other" => ContactPointSystem::Other,
                    _ => return None,
                };

                let use_type = cp.use_type.as_ref().and_then(|u| match u.as_str() {
                    "Home" => Some(ContactPointUse::Home),
                    "Work" => Some(ContactPointUse::Work),
                    "Temp" => Some(ContactPointUse::Temp),
                    "Old" => Some(ContactPointUse::Old),
                    "Mobile" => Some(ContactPointUse::Mobile),
                    _ => None,
                });

                Some(ContactPoint {
                    system,
                    value: cp.value.clone(),
                    use_type,
                })
            })
            .collect()
    }

    /// Reassembles a domain [`Worker`] from its database rows, parsing the
    /// string-stored enums (gender, worker type, name/identifier/contact/link
    /// use types) back into their typed forms. Requires a primary name row;
    /// returns a [`crate::Error::Validation`] if none is present. Unparseable
    /// contact/link tags are dropped (`filter_map`) rather than erroring. The
    /// `documents` / `emergency_contacts` / `photo` collections are left empty
    /// here and populated separately by
    /// [`load_extra_collections`](Self::load_extra_collections).
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Validation`] if `db_names` contains no primary
    /// name row.
    fn from_db_models(
        db_worker: workers::Model,
        db_names: &[worker_names::Model],
        db_identifiers: &[worker_identifiers::Model],
        db_addresses: &[worker_addresses::Model],
        db_contacts: &[worker_contacts::Model],
        db_links: &[worker_links::Model],
    ) -> Result<Worker> {
        use crate::models::{Gender, LinkType};

        // Parse gender. The DB stores lowercase per the CHECK constraint
        // ('male'/'female'/'other'/'unknown'); accept PascalCase too, so
        // rows written by the older `format!("{:?}", …)` path (which
        // reached deployments whose schema lacked the constraint) still
        // round-trip instead of silently reading back as `Unknown`.
        let gender = match db_worker.gender.to_lowercase().as_str() {
            "male" => Gender::Male,
            "female" => Gender::Female,
            "other" => Gender::Other,
            _ => Gender::Unknown,
        };

        // Parse worker_type
        let worker_type = db_worker.worker_type.as_deref().and_then(|wt| match wt {
            "doctor" => Some(WorkerType::Doctor),
            "nurse" => Some(WorkerType::Nurse),
            "carer" => Some(WorkerType::Carer),
            "staff" => Some(WorkerType::Staff),
            "employee" => Some(WorkerType::Employee),
            "manager" => Some(WorkerType::Manager),
            "supervisor" => Some(WorkerType::Supervisor),
            "consultant" => Some(WorkerType::Consultant),
            "other" => Some(WorkerType::Other),
            _ => None,
        });

        // Get primary name
        let primary_name = db_names
            .iter()
            .find(|n| n.is_primary)
            .ok_or_else(|| crate::Error::Validation("Worker has no primary name".to_string()))?;

        let name = Self::human_name_from_db(primary_name);

        // Additional names
        let additional_names = db_names
            .iter()
            .filter(|n| !n.is_primary)
            .map(Self::human_name_from_db)
            .collect();

        let identifiers = Self::identifiers_from_db(db_identifiers);

        // Addresses
        let addresses = db_addresses
            .iter()
            .map(|addr| Address {
                use_type: None,
                line1: addr.line1.clone(),
                line2: addr.line2.clone(),
                city: addr.city.clone(),
                state: addr.state.clone(),
                postal_code: addr.postal_code.clone(),
                country: addr.country.clone(),
            })
            .collect();

        let telecom = Self::telecom_from_db(db_contacts);

        // Links
        let links = db_links
            .iter()
            .filter_map(|link| {
                let link_type = match link.link_type.as_str() {
                    "ReplacedBy" => LinkType::ReplacedBy,
                    "Replaces" => LinkType::Replaces,
                    "Refer" => LinkType::Refer,
                    "Seealso" => LinkType::Seealso,
                    _ => return None,
                };

                Some(WorkerLink {
                    other_worker_id: link.other_worker_id,
                    link_type,
                })
            })
            .collect();

        Ok(Worker {
            id: db_worker.id,
            identifiers,
            active: db_worker.active,
            name,
            additional_names,
            telecom,
            gender,
            worker_type,
            birth_date: db_worker.birth_date.map(time_to_date),
            deceased: db_worker.deceased,
            deceased_datetime: db_worker.deceased_datetime.map(offset_to_ts),
            addresses,
            marital_status: db_worker.marital_status,
            multiple_birth: db_worker.multiple_birth,
            tax_id: db_worker.tax_id,
            // Loaded separately from child tables by `load_extra_collections`.
            documents: vec![],
            emergency_contacts: vec![],
            photo: vec![],
            managing_organization: db_worker.managing_organization_id,
            links,
            created_at: offset_to_ts(db_worker.created_at),
            updated_at: offset_to_ts(db_worker.updated_at),
        })
    }

    /// Fetches the child rows (names, identifiers, addresses, contacts, links)
    /// belonging to `worker_id` in a fixed tuple order matching
    /// [`from_db_models`](Self::from_db_models).
    ///
    /// Runs one `find` per child table; the tuple order is the contract
    /// `from_db_models` relies on, so keep them in lockstep.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the child-table queries fail.
    async fn load_associations(
        &self,
        worker_id: &Uuid,
    ) -> Result<(
        Vec<worker_names::Model>,
        Vec<worker_identifiers::Model>,
        Vec<worker_addresses::Model>,
        Vec<worker_contacts::Model>,
        Vec<worker_links::Model>,
    )> {
        let db_names = worker_names::Entity::find()
            .filter(worker_names::Column::WorkerId.eq(*worker_id))
            .all(&self.db)
            .await?;

        let db_identifiers = worker_identifiers::Entity::find()
            .filter(worker_identifiers::Column::WorkerId.eq(*worker_id))
            .all(&self.db)
            .await?;

        let db_addresses = worker_addresses::Entity::find()
            .filter(worker_addresses::Column::WorkerId.eq(*worker_id))
            .all(&self.db)
            .await?;

        let db_contacts = worker_contacts::Entity::find()
            .filter(worker_contacts::Column::WorkerId.eq(*worker_id))
            .all(&self.db)
            .await?;

        let db_links = worker_links::Entity::find()
            .filter(worker_links::Column::WorkerId.eq(*worker_id))
            .all(&self.db)
            .await?;

        Ok((
            db_names,
            db_identifiers,
            db_addresses,
            db_contacts,
            db_links,
        ))
    }

    /// Load the normalized document / emergency-contact / photo child rows
    /// for `worker.id` and populate them onto `worker`.
    ///
    /// The companion to [`insert_extra_collections`]: each collection is read
    /// back ordered by its `position` column so list order is preserved, and
    /// emergency-contact telecom rows are fetched per contact. String tags are
    /// decoded with [`tag_to_enum`], falling back to the `Other` variant for an
    /// unknown tag. An emergency-contact address is reconstructed only when at
    /// least one flattened address column is present.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the child-table queries fail.
    async fn load_extra_collections(&self, worker: &mut Worker) -> Result<()> {
        let id = worker.id;

        let doc_rows = worker_documents::Entity::find()
            .filter(worker_documents::Column::WorkerId.eq(id))
            .order_by_asc(worker_documents::Column::Position)
            .all(&self.db)
            .await?;
        worker.documents = doc_rows
            .into_iter()
            .map(|r| IdentityDocument {
                document_type: tag_to_enum(Some(r.document_type.as_str()))
                    .unwrap_or(DocumentType::Other),
                number: r.number,
                issuing_country: r.issuing_country,
                issuing_authority: r.issuing_authority,
                issue_date: r.issue_date.map(time_to_date),
                expiry_date: r.expiry_date.map(time_to_date),
                verified: r.verified,
            })
            .collect();

        let ec_rows = worker_emergency_contacts::Entity::find()
            .filter(worker_emergency_contacts::Column::WorkerId.eq(id))
            .order_by_asc(worker_emergency_contacts::Column::Position)
            .all(&self.db)
            .await?;
        let mut emergency_contacts = Vec::with_capacity(ec_rows.len());
        for ec in ec_rows {
            let tel_rows = worker_emergency_contact_telecom::Entity::find()
                .filter(worker_emergency_contact_telecom::Column::EmergencyContactId.eq(ec.id))
                .order_by_asc(worker_emergency_contact_telecom::Column::Position)
                .all(&self.db)
                .await?;
            let telecom = tel_rows
                .into_iter()
                .map(|t| ContactPoint {
                    system: tag_to_enum(Some(t.system.as_str()))
                        .unwrap_or(ContactPointSystem::Other),
                    value: t.value,
                    use_type: tag_to_enum(t.use_type.as_deref()),
                })
                .collect();
            let has_address = ec.address_use_type.is_some()
                || ec.address_line1.is_some()
                || ec.address_line2.is_some()
                || ec.address_city.is_some()
                || ec.address_state.is_some()
                || ec.address_postal_code.is_some()
                || ec.address_country.is_some();
            let address = has_address.then(|| Address {
                use_type: tag_to_enum(ec.address_use_type.as_deref()),
                line1: ec.address_line1,
                line2: ec.address_line2,
                city: ec.address_city,
                state: ec.address_state,
                postal_code: ec.address_postal_code,
                country: ec.address_country,
            });
            emergency_contacts.push(EmergencyContact {
                name: ec.name,
                relationship: ec.relationship,
                telecom,
                address,
                is_primary: ec.is_primary,
            });
        }
        worker.emergency_contacts = emergency_contacts;

        let photo_rows = worker_photos::Entity::find()
            .filter(worker_photos::Column::WorkerId.eq(id))
            .order_by_asc(worker_photos::Column::Position)
            .all(&self.db)
            .await?;
        worker.photo = photo_rows.into_iter().map(|r| r.url).collect();

        Ok(())
    }
}

// SeaORM implementation of the repository trait. Each mutation runs its
// multi-table writes inside a transaction, then (best-effort) publishes an
// event and records an audit entry after the commit.
#[async_trait::async_trait]
impl WorkerRepository for SeaOrmWorkerRepository {
    async fn create(&self, worker: &Worker) -> Result<Worker> {
        // All inserts share one transaction so a partial worker is never left.
        let txn = self.db.begin().await?;

        let (new_worker, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            Self::to_active_models(worker)?;

        // Insert worker
        let db_worker = new_worker.insert(&txn).await?;

        // Insert names
        for name in new_names {
            name.insert(&txn).await?;
        }

        // Insert identifiers
        for identifier in new_identifiers {
            identifier.insert(&txn).await?;
        }

        // Insert addresses
        for address in new_addresses {
            address.insert(&txn).await?;
        }

        // Insert contacts
        for contact in new_contacts {
            contact.insert(&txn).await?;
        }

        // Insert links
        for link in new_links {
            link.insert(&txn).await?;
        }

        // Insert documents / emergency contacts / photos (normalized).
        insert_extra_collections(&txn, worker).await?;

        // Durable event bus (Phase 2): under the outbox transport, write the
        // `event_outbox` row **inside this same transaction**, before the
        // commit, so the entity rows and the event commit atomically (or roll
        // back together). A no-op under the memory transport, which keeps the
        // post-commit ring-buffer publish below.
        self.enqueue_outbox(&txn, worker, EventKind::Created)
            .await?;

        // Commit closes the transaction boundary: everything above is now
        // durable. The reload + event + audit steps below run post-commit and
        // are best-effort (a streaming/audit failure must not undo the write).
        txn.commit().await?;

        // Reload from the DB so the returned record reflects exactly what was
        // persisted (server-assigned timestamps, normalized child rows).
        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(&db_worker.id).await?;

        let mut result = Self::from_db_models(
            db_worker,
            &db_names,
            &db_identifiers,
            &db_addresses,
            &db_contacts,
            &db_links,
        )?;
        self.load_extra_collections(&mut result).await?;

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Created {
            worker: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Ok(worker_json) = serde_json::to_value(&result) {
            self.log_audit(
                "CREATE",
                result.id,
                None,
                Some(worker_json),
                &AuditContext::default(),
            )
            .await;
        }

        Ok(result)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Worker>> {
        let db_worker = workers::Entity::find_by_id(*id)
            .filter(workers::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;

        let Some(db_worker) = db_worker else {
            return Ok(None);
        };

        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(id).await?;

        let mut worker = Self::from_db_models(
            db_worker,
            &db_names,
            &db_identifiers,
            &db_addresses,
            &db_contacts,
            &db_links,
        )?;
        self.load_extra_collections(&mut worker).await?;
        Ok(Some(worker))
    }

    async fn update(&self, worker: &Worker) -> Result<Worker> {
        // Capture the pre-update state for the audit diff.
        let old_worker = self.get_by_id(&worker.id).await?;

        // Update is implemented as "update parent row + replace all child
        // rows" within a single transaction.
        let txn = self.db.begin().await?;

        // Update worker. This duplicates `apply_worker_row_replacement`
        // above rather than calling it — pre-existing drift, left alone
        // here beyond adding the rehash it also needs.
        let content_hash = content_hash_for_update(&txn, worker).await?;
        let update_model = workers::ActiveModel {
            id: Set(worker.id),
            active: Set(worker.active),
            content_hash: Set(Some(content_hash)),
            worker_type: Set(worker
                .worker_type
                .as_ref()
                .map(std::string::ToString::to_string)),
            // Lowercased for the `workers_gender_check` CHECK constraint.
            gender: Set(format!("{:?}", worker.gender).to_lowercase()),
            birth_date: Set(worker.birth_date.map(date_to_time)),
            tax_id: Set(worker.tax_id.clone()),
            deceased: Set(worker.deceased),
            deceased_datetime: Set(worker.deceased_datetime.map(ts_to_offset)),
            marital_status: Set(worker.marital_status.clone()),
            multiple_birth: Set(worker.multiple_birth),
            managing_organization_id: Set(worker.managing_organization),
            updated_at: Set(OffsetDateTime::now_utc()),
            updated_by: Set(None),
            ..Default::default()
        };
        update_model.update(&txn).await?;

        // Delete existing associated data
        worker_names::Entity::delete_many()
            .filter(worker_names::Column::WorkerId.eq(worker.id))
            .exec(&txn)
            .await?;

        worker_identifiers::Entity::delete_many()
            .filter(worker_identifiers::Column::WorkerId.eq(worker.id))
            .exec(&txn)
            .await?;

        worker_addresses::Entity::delete_many()
            .filter(worker_addresses::Column::WorkerId.eq(worker.id))
            .exec(&txn)
            .await?;

        worker_contacts::Entity::delete_many()
            .filter(worker_contacts::Column::WorkerId.eq(worker.id))
            .exec(&txn)
            .await?;

        worker_links::Entity::delete_many()
            .filter(worker_links::Column::WorkerId.eq(worker.id))
            .exec(&txn)
            .await?;

        // Deleting emergency contacts cascades to their telecom rows.
        worker_emergency_contacts::Entity::delete_many()
            .filter(worker_emergency_contacts::Column::WorkerId.eq(worker.id))
            .exec(&txn)
            .await?;
        worker_documents::Entity::delete_many()
            .filter(worker_documents::Column::WorkerId.eq(worker.id))
            .exec(&txn)
            .await?;
        worker_photos::Entity::delete_many()
            .filter(worker_photos::Column::WorkerId.eq(worker.id))
            .exec(&txn)
            .await?;

        // Re-insert associated data
        let (_, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            Self::to_active_models(worker)?;

        for name in new_names {
            name.insert(&txn).await?;
        }
        for identifier in new_identifiers {
            identifier.insert(&txn).await?;
        }
        for address in new_addresses {
            address.insert(&txn).await?;
        }
        for contact in new_contacts {
            contact.insert(&txn).await?;
        }
        for link in new_links {
            link.insert(&txn).await?;
        }
        insert_extra_collections(&txn, worker).await?;

        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&txn, worker, EventKind::Updated)
            .await?;

        txn.commit().await?;

        // Fetch and return updated worker
        let result = self
            .get_by_id(&worker.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Worker not found after update".to_string()))?;

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Updated {
            worker: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_json) = old_worker
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
            && let Ok(new_json) = serde_json::to_value(&result)
        {
            self.log_audit(
                "UPDATE",
                result.id,
                Some(old_json),
                Some(new_json),
                &AuditContext::default(),
            )
            .await;
        }

        Ok(result)
    }

    async fn merge(&self, survivor: &Worker, duplicate_id: &Uuid) -> Result<Worker> {
        // Snapshots for the audit trail: the survivor's pre-image for the
        // UPDATE row, the duplicate's pre-image for the DELETE row (and to
        // build the duplicate's `Deleted` outbox envelope).
        let old_survivor = self.get_by_id(&survivor.id).await?;
        let old_duplicate = self.get_by_id(duplicate_id).await?;

        // The whole merge — survivor row updates + duplicate soft-delete +
        // both outbox rows — commits (or rolls back) under one transaction.
        let txn = self.db.begin().await?;

        // Apply the survivor's parent + child rows (parent update + wholesale
        // child-row replacement), shared with the `update` sequence.
        apply_worker_row_replacement(&txn, survivor).await?;

        // Soft-delete the duplicate on the same transaction.
        // A soft delete changes the lifecycle state the digest binds, so
        // it must be rehashed or every deleted record would read as
        // tampered. The pre-image loaded above for the audit trail doubles
        // as the record to hash.
        let deleted_at = OffsetDateTime::now_utc();
        let deleted_micros = i64::try_from(deleted_at.unix_timestamp_nanos() / 1_000).ok();
        let tombstone_hash = old_duplicate
            .as_ref()
            .map(|w| crate::compliance::record_integrity::hash_with_deleted_at(w, deleted_micros))
            .transpose()?;
        let dup_delete = workers::ActiveModel {
            id: Set(*duplicate_id),
            deleted_at: Set(Some(deleted_at)),
            deleted_by: Set(Some("system".to_string())),
            content_hash: tombstone_hash.map_or(sea_orm::ActiveValue::NotSet, |h| Set(Some(h))),
            ..Default::default()
        };
        dup_delete.update(&txn).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the survivor
        // (carrying the duplicate's pid via `merged_from`, so a merge-repointing
        // consumer can move edges off the duplicate) and a `Deleted` outbox row
        // for the duplicate — both inside this transaction. A no-op under the
        // memory transport.
        if self.transport.is_outbox() {
            OutboxInsert::for_merge(survivor, duplicate_id)?
                .insert_on(&txn)
                .await?;
            if let Some(dup) = old_duplicate.as_ref() {
                OutboxInsert::for_event(dup, EventKind::Deleted)?
                    .insert_on(&txn)
                    .await?;
            }
        }

        txn.commit().await?;

        let result = self
            .get_by_id(&survivor.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Worker not found after merge".to_string()))?;

        // Post-commit in-memory stream (memory transport keeps this): a `Merged`
        // for the survivor and a `Deleted` for the duplicate.
        self.publish_event(crate::streaming::WorkerEvent::Merged {
            source_id: *duplicate_id,
            target_id: result.id,
            timestamp: chrono::Utc::now(),
        });
        self.publish_event(crate::streaming::WorkerEvent::Deleted {
            worker_id: *duplicate_id,
            timestamp: chrono::Utc::now(),
        });

        // Audit rows: an UPDATE trail for the survivor and a DELETE trail for
        // the duplicate.
        if let Some(old_json) = old_survivor
            .as_ref()
            .and_then(|p| serde_json::to_value(p).ok())
            && let Ok(new_json) = serde_json::to_value(&result)
        {
            self.log_audit(
                "UPDATE",
                result.id,
                Some(old_json),
                Some(new_json),
                &AuditContext::default(),
            )
            .await;
        }
        if let Some(dup) = old_duplicate
            && let Ok(dup_json) = serde_json::to_value(&dup)
        {
            self.log_audit(
                "DELETE",
                *duplicate_id,
                Some(dup_json),
                None,
                &AuditContext::default(),
            )
            .await;
        }

        Ok(result)
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        // Capture the pre-delete state for the audit record.
        let old_worker = self.get_by_id(id).await?;

        // Soft delete: stamp deleted_at/deleted_by rather than removing rows.
        // A soft delete changes the lifecycle state the digest binds, so
        // it must be rehashed or every deleted record would read as
        // tampered. The pre-image loaded above for the audit trail doubles
        // as the record to hash.
        let deleted_at = OffsetDateTime::now_utc();
        let deleted_micros = i64::try_from(deleted_at.unix_timestamp_nanos() / 1_000).ok();
        let tombstone_hash = old_worker
            .as_ref()
            .map(|w| crate::compliance::record_integrity::hash_with_deleted_at(w, deleted_micros))
            .transpose()?;
        let update_model = workers::ActiveModel {
            id: Set(*id),
            deleted_at: Set(Some(deleted_at)),
            deleted_by: Set(Some("system".to_string())),
            content_hash: tombstone_hash.map_or(sea_orm::ActiveValue::NotSet, |h| Set(Some(h))),
            ..Default::default()
        };
        // Unlike create/update, the soft-delete is a single row update with no
        // existing transaction. Under the outbox transport we open one here so
        // the tombstone write and the `deleted` outbox row commit atomically;
        // the memory transport keeps the plain, tx-free update.
        if self.transport.is_outbox() {
            let txn = self.db.begin().await?;
            update_model.update(&txn).await?;
            if let Some(old) = old_worker.as_ref() {
                self.enqueue_outbox(&txn, old, EventKind::Deleted).await?;
            }
            txn.commit().await?;
        } else {
            update_model.update(&self.db).await?;
        }

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Deleted {
            worker_id: *id,
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_worker) = old_worker
            && let Ok(old_json) = serde_json::to_value(&old_worker)
        {
            self.log_audit(
                "DELETE",
                *id,
                Some(old_json),
                None,
                &AuditContext::default(),
            )
            .await;
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Worker>> {
        // Case-insensitive substring match on family name via SQL LIKE.
        // SEC-G4: escape the caller's `LIKE` metacharacters (`%`, `_`, `\`)
        // before wrapping in the `%…%` contains-pattern, so a query of
        // `%` (matches every row) or `_`×N (an expensive scan) is treated
        // literally, not as a wildcard. The value is a bound parameter
        // (no SQL injection); this closes the wildcard-injection / DoS
        // vector. Postgres `LIKE` uses `\` as the escape char.
        let search_pattern = format!("%{}%", escape_like(&query.to_lowercase()));

        // First collect distinct matching worker IDs, then hydrate each.
        let worker_ids: Vec<Uuid> = worker_names::Entity::find()
            .filter(Expr::cust_with_values(
                "LOWER(family) LIKE $1",
                [search_pattern],
            ))
            .select_only()
            .column(worker_names::Column::WorkerId)
            .distinct()
            .into_tuple()
            .all(&self.db)
            .await?;

        // Hydrate each match through `get_by_id` (one query set per worker);
        // this also re-applies the soft-delete filter, so a name matching a
        // soft-deleted worker is skipped.
        let mut workers = Vec::new();
        for worker_id in worker_ids {
            if let Some(worker) = self.get_by_id(&worker_id).await? {
                workers.push(worker);
            }
        }

        Ok(workers)
    }

    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Worker>> {
        let db_workers: Vec<workers::Model> = workers::Entity::find()
            .filter(workers::Column::DeletedAt.is_null())
            .filter(workers::Column::Active.eq(true))
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await?;

        // Page the parent rows in SQL (limit/offset), then hydrate each into a
        // full domain worker via `get_by_id`.
        let mut workers = Vec::new();
        for db_worker in db_workers {
            if let Some(worker) = self.get_by_id(&db_worker.id).await? {
                workers.push(worker);
            }
        }

        Ok(workers)
    }
}

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path. They
/// require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped by a
/// bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE under a
/// bare `cargo test --lib`.
/// Escape SQL `LIKE` wildcards (`\`, `%`, `_`) so a user query matches
/// literally inside a `%…%` contains-pattern (SEC-G4). The backslash is
/// escaped first so it cannot re-enable a following wildcard. Postgres
/// `LIKE` uses `\` as the default escape character.
fn escape_like(q: &str) -> String {
    q.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    use super::{SeaOrmWorkerRepository, WorkerRepository, escape_like};
    /// SEC-G4: `LIKE` wildcards in a search query are neutralised so they
    /// match literally and cannot scan every row (`%`) or force pathological
    /// work (`_`). Pure, DB-free.
    #[test]
    fn escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("smith"), "smith");
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        // Backslash is escaped first so it can't re-enable a wildcard.
        assert_eq!(escape_like("a\\%"), "a\\\\\\%");
    }

    /// The `workers.gender` CHECK constraint's vocabulary, copied from
    /// `migrations/2024122800000002_create_workers/up.sql`. A value
    /// outside this set is rejected by `PostgreSQL` at insert time.
    const DB_GENDER_TOKENS: [&str; 4] = ["male", "female", "other", "unknown"];

    /// Every [`Gender`] variant is persisted as a token the
    /// `workers_gender_check` CHECK constraint accepts.
    ///
    /// Regression pin: `to_active_models` (and the two sibling write
    /// paths) previously stored the bare `Debug` form — `"Unknown"`,
    /// `"Male"` — which `PostgreSQL` rejects, so **every** create and
    /// update against a constrained schema failed with
    /// `violates check constraint "workers_gender_check"`. Pure and
    /// DB-free, so the vocabulary is pinned on a bare `cargo test --lib`
    /// rather than only when a database happens to be reachable.
    #[test]
    fn gender_is_persisted_as_a_constraint_legal_token() {
        for gender in [Gender::Male, Gender::Female, Gender::Other, Gender::Unknown] {
            let worker = Worker::new(
                HumanName {
                    use_type: None,
                    family: "Doe".into(),
                    given: vec!["Jane".into()],
                    prefix: vec![],
                    suffix: vec![],
                },
                gender,
            );
            let (row, ..) = SeaOrmWorkerRepository::to_active_models(&worker).expect("hashable");
            let stored = row.gender.unwrap();
            assert!(
                DB_GENDER_TOKENS.contains(&stored.as_str()),
                "{gender:?} persisted as {stored:?}, which the CHECK constraint \
                 rejects (allowed: {DB_GENDER_TOKENS:?})"
            );
            // And it is the serde wire token, so the DB, the search index,
            // and the FHIR surface all agree on one spelling.
            assert_eq!(
                stored,
                serde_json::to_value(gender)
                    .expect("gender serializes")
                    .as_str()
                    .expect("as a string")
            );
        }
    }

    use crate::db::models::event_outbox;
    use crate::models::{Gender, HumanName, Worker};
    use crate::streaming::EventTransport;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    async fn connect() -> sea_orm::DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    fn a_worker(family: &str) -> Worker {
        let name = HumanName {
            use_type: None,
            family: family.to_string(),
            given: vec!["Test".into()],
            prefix: vec![],
            suffix: vec![],
        };
        Worker::new(name, Gender::Unknown)
    }

    /// Create writes, in one transaction: exactly one `created` outbox row
    /// for the new worker under the outbox transport.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = SeaOrmWorkerRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let worker = repo.create(&a_worker("Created")).await.unwrap();

        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(worker.id))
            .filter(event_outbox::Column::Kind.eq("created"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1, "one created outbox row for the new worker");
        assert_eq!(rows[0].entity, "worker");
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = SeaOrmWorkerRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo.create(&a_worker("Survivor")).await.unwrap();
        let duplicate = repo.create(&a_worker("Duplicate")).await.unwrap();

        let merged = repo.merge(&survivor, &duplicate.id).await.unwrap();
        assert_eq!(merged.id, survivor.id);

        // Survivor: exactly one `merged` row, carrying the duplicate pid.
        let merged_rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(survivor.id))
            .filter(event_outbox::Column::Kind.eq("merged"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(merged_rows.len(), 1, "one merged outbox row for survivor");
        assert_eq!(
            merged_rows[0].payload["merged_from"],
            serde_json::json!(duplicate.id.to_string()),
            "merged row carries the duplicate's pid"
        );

        // Duplicate: a `deleted` row.
        let deleted_rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(duplicate.id))
            .filter(event_outbox::Column::Kind.eq("deleted"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(
            deleted_rows.len(),
            1,
            "one deleted outbox row for duplicate"
        );
    }
}
