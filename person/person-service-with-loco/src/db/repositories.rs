//! Person persistence via the repository pattern.
//!
//! [`PersonRepository`] is the storage-agnostic trait the rest of the
//! service depends on; [`SeaOrmPersonRepository`] is the PostgreSQL
//! implementation. The implementation maps the rich domain [`Person`](crate::models::Person)
//! to/from the normalized child tables (names, identifiers, addresses,
//! contacts, links), wraps multi-table writes in a transaction, performs
//! soft deletes, and — when configured — publishes a
//! [`PersonEvent`](crate::streaming::PersonEvent) and writes an audit row
//! for every mutation. [`AuditContext`] carries who/where provenance
//! into those audit rows.

use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use time::OffsetDateTime;
use uuid::Uuid;

use super::convert::{date_to_time, offset_to_ts, time_to_date, ts_to_offset};

use super::models::{
    person_addresses, person_contacts, person_documents, person_emergency_contact_telecom,
    person_emergency_contacts, person_identifiers, person_links, person_names, person_photos,
    persons,
};
use crate::Result;
use crate::db::outbox::OutboxInsert;
use crate::models::{
    Address, ContactPoint, ContactPointSystem, DocumentType, EmergencyContact, HumanName,
    Identifier, IdentityDocument, Person, PersonLink,
};
use crate::streaming::{EventKind, EventTransport};

/// Serialize a fieldless enum to its canonical serde string tag (used for
/// the document/telecom/address enum columns).
fn enum_to_tag<T: serde::Serialize>(value: &T) -> Option<String> {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
}

/// Parse a stored string tag back into a fieldless enum, or `None` if it is
/// absent or unrecognized.
fn tag_to_enum<T: serde::de::DeserializeOwned>(tag: Option<&String>) -> Option<T> {
    let t = tag?;
    serde_json::from_value(serde_json::Value::String(t.clone())).ok()
}

/// Insert the normalized document / emergency-contact (+ telecom) / photo
/// child rows for `person` on connection `conn`.
async fn insert_extra_collections<C: sea_orm::ConnectionTrait>(
    conn: &C,
    person: &Person,
) -> Result<()> {
    for (i, doc) in person.documents.iter().enumerate() {
        person_documents::ActiveModel {
            id: Set(Uuid::new_v4()),
            person_id: Set(person.id),
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
    for (i, ec) in person.emergency_contacts.iter().enumerate() {
        let ec_id = Uuid::new_v4();
        let addr = ec.address.as_ref();
        person_emergency_contacts::ActiveModel {
            id: Set(ec_id),
            person_id: Set(person.id),
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
            person_emergency_contact_telecom::ActiveModel {
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
    for (i, url) in person.photo.iter().enumerate() {
        person_photos::ActiveModel {
            id: Set(Uuid::new_v4()),
            person_id: Set(person.id),
            url: Set(url.clone()),
            position: Set(i32::try_from(i).unwrap_or(i32::MAX)),
        }
        .insert(conn)
        .await?;
    }
    Ok(())
}

/// Apply a person's parent-row update and wholesale child-row replacement
/// on `conn`. Shared by [`PersonRepository::update`] and
/// [`PersonRepository::merge`]: preserves `created_*`, stamps `updated_at`
/// to now, deletes every child row for the person, then re-inserts them
/// from the domain model. Runs on the caller's connection/transaction so
/// it shares that commit boundary.
async fn apply_update_rows<C: ConnectionTrait>(conn: &C, person: &Person) -> Result<()> {
    // Update the parent `persons` row (only scalar fields; `..Default`
    // leaves created_* untouched).
    let update_model = persons::ActiveModel {
        id: Set(person.id),
        active: Set(person.active),
        // DB CHECK constraint enforces lowercase ('male'/'female'/'other'/'unknown');
        // Gender's serde rename_all="lowercase" produces the same shape.
        gender: Set(format!("{:?}", person.gender).to_lowercase()),
        birth_date: Set(person.birth_date.map(date_to_time)),
        tax_id: Set(person.tax_id.clone()),
        deceased: Set(person.deceased),
        deceased_datetime: Set(person.deceased_datetime.map(ts_to_offset)),
        marital_status: Set(person.marital_status.clone()),
        multiple_birth: Set(person.multiple_birth),
        managing_organization_id: Set(person.managing_organization),
        updated_at: Set(OffsetDateTime::now_utc()),
        updated_by: Set(None),
        ..Default::default()
    };
    update_model.update(conn).await?;

    // Delete existing associated child rows.
    person_names::Entity::delete_many()
        .filter(person_names::Column::PersonId.eq(person.id))
        .exec(conn)
        .await?;
    person_identifiers::Entity::delete_many()
        .filter(person_identifiers::Column::PersonId.eq(person.id))
        .exec(conn)
        .await?;
    person_addresses::Entity::delete_many()
        .filter(person_addresses::Column::PersonId.eq(person.id))
        .exec(conn)
        .await?;
    person_contacts::Entity::delete_many()
        .filter(person_contacts::Column::PersonId.eq(person.id))
        .exec(conn)
        .await?;
    person_links::Entity::delete_many()
        .filter(person_links::Column::PersonId.eq(person.id))
        .exec(conn)
        .await?;
    // Deleting emergency contacts cascades to their telecom rows.
    person_emergency_contacts::Entity::delete_many()
        .filter(person_emergency_contacts::Column::PersonId.eq(person.id))
        .exec(conn)
        .await?;
    person_documents::Entity::delete_many()
        .filter(person_documents::Column::PersonId.eq(person.id))
        .exec(conn)
        .await?;
    person_photos::Entity::delete_many()
        .filter(person_photos::Column::PersonId.eq(person.id))
        .exec(conn)
        .await?;

    // Re-insert associated child rows.
    let (_, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
        SeaOrmPersonRepository::to_active_models(person);
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
    insert_extra_collections(conn, person).await?;
    Ok(())
}

/// Soft-delete the person `id` on `conn` by stamping `deleted_at` /
/// `deleted_by` and leaving every other column (and the child rows) in
/// place. Factored from [`PersonRepository::delete`]'s soft-delete so
/// [`PersonRepository::merge`] can reuse it on its own transaction.
async fn apply_soft_delete_row<C: ConnectionTrait>(conn: &C, id: &Uuid) -> Result<()> {
    let row = persons::ActiveModel {
        id: Set(*id),
        deleted_at: Set(Some(OffsetDateTime::now_utc())),
        deleted_by: Set(Some("system".to_string())),
        ..Default::default()
    };
    row.update(conn).await?;
    Ok(())
}

/// The active-model tuple produced by exploding a [`Person`] into the
/// parent row plus its child-table rows, ready for insertion.
type PersonActiveModels = (
    persons::ActiveModel,
    Vec<person_names::ActiveModel>,
    Vec<person_identifiers::ActiveModel>,
    Vec<person_addresses::ActiveModel>,
    Vec<person_contacts::ActiveModel>,
    Vec<person_links::ActiveModel>,
);

/// Request provenance attached to audit-log writes.
///
/// Defaults to a `"system"` actor with no network metadata; handlers
/// populate it from the inbound HTTP request when available.
#[derive(Debug, Clone)]
pub struct AuditContext {
    /// Authenticated user id, or `None` for anonymous/system actions.
    pub user_id: Option<String>,
    /// Originating client IP address, if known.
    pub ip_address: Option<String>,
    /// Originating `User-Agent` header, if known.
    pub user_agent: Option<String>,
}

impl Default for AuditContext {
    /// A `"system"` actor with no IP or user-agent.
    fn default() -> Self {
        Self {
            user_id: Some("system".to_string()),
            ip_address: None,
            user_agent: None,
        }
    }
}

/// Storage-agnostic CRUD + search interface for [`Person`] records.
///
/// `Send + Sync` so it can be shared as `Arc<dyn PersonRepository>`
/// across async handlers.
#[async_trait::async_trait]
pub trait PersonRepository: Send + Sync {
    /// Persist a new person (and its child rows) and return the stored form.
    async fn create(&self, person: &Person) -> Result<Person>;

    /// Fetch a non-deleted person by id, or `None` if absent/soft-deleted.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Person>>;

    /// Replace a person and all its child rows, returning the new state.
    async fn update(&self, person: &Person) -> Result<Person>;

    /// Merge the `duplicate_id` record into `survivor`: apply the
    /// survivor's parent + child row updates and soft-delete the duplicate
    /// **in one transaction**, so the whole merge commits (or rolls back)
    /// atomically. Under the outbox transport this enqueues a `Merged`
    /// outbox row for the survivor (carrying the duplicate's pid via
    /// `merged_from`) and a `Deleted` outbox row for the duplicate on the
    /// same transaction. Returns the reloaded survivor.
    async fn merge(&self, survivor: &Person, duplicate_id: &Uuid) -> Result<Person>;

    /// Soft-delete a person (sets `deleted_at`; row is retained).
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// Find persons whose family name matches `query` (case-insensitive).
    async fn search(&self, query: &str) -> Result<Vec<Person>>;

    /// Page through active, non-deleted persons via `limit`/`offset`.
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Person>>;
}

/// `PostgreSQL` [`PersonRepository`] backed by `SeaORM`.
///
/// Optionally fans out to an event publisher and audit log; both are set
/// via the `with_*` builder methods and are no-ops when absent.
pub struct SeaOrmPersonRepository {
    /// The `SeaORM` connection (cheap to clone, internally pooled).
    db: DatabaseConnection,
    /// Optional sink for [`PersonEvent`](crate::streaming::PersonEvent)s.
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    /// Optional audit-trail writer.
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
    /// Which event transport is active (durable event bus, Phase 2).
    /// [`EventTransport::Memory`] (default) keeps the legacy post-commit
    /// in-memory publish; [`EventTransport::Outbox`] additionally writes
    /// one `event_outbox` row **inside** each write's transaction.
    transport: EventTransport,
}

impl SeaOrmPersonRepository {
    /// Build a repository over `db` with no event/audit sinks attached and
    /// the default [`EventTransport::Memory`] transport (behaviour
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
    /// `AppState` wires this from `PERSON_EVENT_TRANSPORT` via
    /// [`crate::streaming::transport`].
    #[must_use]
    pub fn with_transport(mut self, transport: EventTransport) -> Self {
        self.transport = transport;
        self
    }

    /// When the outbox transport is active, enqueue one `event_outbox`
    /// row for `kind` applied to `person` **on `conn`** — pass the open
    /// `&DatabaseTransaction` so the row commits with the entity write
    /// (the outbox atomicity guarantee). A no-op under
    /// [`EventTransport::Memory`].
    ///
    /// The `ConnectionTrait` generic lives here on a concrete method (not
    /// on the object-safe [`PersonRepository`] trait), which is how a
    /// `dyn`-trait repository threads the outbox insert into its own
    /// transaction.
    async fn enqueue_outbox<C: ConnectionTrait>(
        &self,
        conn: &C,
        person: &Person,
        kind: EventKind,
    ) -> Result<()> {
        if self.transport.is_outbox() {
            OutboxInsert::for_event(person, kind)?
                .insert_on(conn)
                .await?;
        }
        Ok(())
    }

    /// Attach an event publisher; returns `self` for chaining.
    #[must_use]
    pub fn with_event_publisher(
        mut self,
        publisher: std::sync::Arc<dyn crate::streaming::EventProducer>,
    ) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Attach an audit-log repository; returns `self` for chaining.
    #[must_use]
    pub fn with_audit_log(
        mut self,
        audit_log: std::sync::Arc<super::audit::AuditLogRepository>,
    ) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// Publish `event` if a publisher is set; log and swallow any error.
    fn publish_event(&self, event: crate::streaming::PersonEvent) {
        if let Some(ref publisher) = self.event_publisher
            && let Err(e) = publisher.publish(event)
        {
            tracing::error!("Failed to publish event: {}", e);
        }
    }

    /// Write an audit row for `action` if an audit log is configured.
    ///
    /// Dispatches to the matching `log_create`/`log_update`/`log_delete`
    /// helper; unknown actions are ignored. Errors are logged, not raised.
    async fn log_audit(
        &self,
        action: &str,
        entity_id: uuid::Uuid,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        context: &AuditContext,
    ) {
        if let Some(ref audit_log) = self.audit_log {
            let result = match action {
                "CREATE" => {
                    audit_log
                        .log_create(
                            "Person",
                            entity_id,
                            new_values.unwrap_or(serde_json::Value::Null),
                            context,
                        )
                        .await
                }
                "UPDATE" => {
                    audit_log
                        .log_update(
                            "Person",
                            entity_id,
                            old_values.unwrap_or(serde_json::Value::Null),
                            new_values.unwrap_or(serde_json::Value::Null),
                            context,
                        )
                        .await
                }
                "DELETE" => {
                    audit_log
                        .log_delete(
                            "Person",
                            entity_id,
                            old_values.unwrap_or(serde_json::Value::Null),
                            context,
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

    /// Explode a domain [`Person`] into the `SeaORM` active models for the
    /// parent row and each child table.
    ///
    /// The primary name gets `is_primary = true`; the first address and
    /// first contact are flagged primary by position. Enum fields are
    /// stringified (`{:?}`) except `gender`, which is lowercased to honor
    /// the DB CHECK constraint.
    /// Build the `person_names` active models: the primary name (flagged
    /// `is_primary = true`) followed by any additional names.
    fn name_active_models(person: &Person) -> Vec<person_names::ActiveModel> {
        let to_model = |name: &HumanName, is_primary: bool| person_names::ActiveModel {
            id: Set(Uuid::new_v4()),
            person_id: Set(person.id),
            use_type: Set(name.use_type.as_ref().map(|u| format!("{u:?}"))),
            family: Set(name.family.clone()),
            given: Set(name.given.clone()),
            prefix: Set(name.prefix.clone()),
            suffix: Set(name.suffix.clone()),
            is_primary: Set(is_primary),
            created_at: Set(OffsetDateTime::now_utc()),
            updated_at: Set(OffsetDateTime::now_utc()),
        };

        let mut names = vec![to_model(&person.name, true)];
        names.extend(person.additional_names.iter().map(|n| to_model(n, false)));
        names
    }

    fn to_active_models(person: &Person) -> PersonActiveModels {
        let new_person = persons::ActiveModel {
            id: Set(person.id),
            active: Set(person.active),
            // DB CHECK constraint enforces lowercase ('male'/'female'/'other'/'unknown');
            // Gender's serde rename_all="lowercase" produces the same shape.
            gender: Set(format!("{:?}", person.gender).to_lowercase()),
            birth_date: Set(person.birth_date.map(date_to_time)),
            tax_id: Set(person.tax_id.clone()),
            deceased: Set(person.deceased),
            deceased_datetime: Set(person.deceased_datetime.map(ts_to_offset)),
            marital_status: Set(person.marital_status.clone()),
            multiple_birth: Set(person.multiple_birth),
            managing_organization_id: Set(person.managing_organization),
            created_at: Set(OffsetDateTime::now_utc()),
            updated_at: Set(OffsetDateTime::now_utc()),
            created_by: Set(None),
            updated_by: Set(None),
            deleted_at: Set(None),
            deleted_by: Set(None),
        };

        let names = Self::name_active_models(person);

        // Identifiers
        let identifiers = person
            .identifiers
            .iter()
            .map(|id| person_identifiers::ActiveModel {
                id: Set(Uuid::new_v4()),
                person_id: Set(person.id),
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
        let addresses = person
            .addresses
            .iter()
            .enumerate()
            .map(|(idx, addr)| person_addresses::ActiveModel {
                id: Set(Uuid::new_v4()),
                person_id: Set(person.id),
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
        let contacts = person
            .telecom
            .iter()
            .enumerate()
            .map(|(idx, cp)| person_contacts::ActiveModel {
                id: Set(Uuid::new_v4()),
                person_id: Set(person.id),
                system: Set(format!("{:?}", cp.system)),
                value: Set(cp.value.clone()),
                use_type: Set(cp.use_type.as_ref().map(|u| format!("{u:?}"))),
                is_primary: Set(idx == 0),
                created_at: Set(OffsetDateTime::now_utc()),
                updated_at: Set(OffsetDateTime::now_utc()),
            })
            .collect();

        // Links
        let links = person
            .links
            .iter()
            .map(|link| person_links::ActiveModel {
                id: Set(Uuid::new_v4()),
                person_id: Set(person.id),
                other_person_id: Set(link.other_person_id),
                link_type: Set(format!("{:?}", link.link_type)),
                created_at: Set(OffsetDateTime::now_utc()),
                created_by: Set(None),
            })
            .collect();

        (new_person, names, identifiers, addresses, contacts, links)
    }

    /// Rebuild a domain [`HumanName`] from a stored `person_names` row,
    /// parsing the stringified `use_type` back to its [`NameUse`](crate::models::NameUse) variant.
    fn human_name_from_db(n: &person_names::Model) -> HumanName {
        use crate::models::NameUse;
        let use_type = n.use_type.as_ref().and_then(|u| match u.as_str() {
            "Usual" => Some(NameUse::Usual),
            "Official" => Some(NameUse::Official),
            "Temp" => Some(NameUse::Temp),
            "Nickname" => Some(NameUse::Nickname),
            "Anonymous" => Some(NameUse::Anonymous),
            "Old" => Some(NameUse::Old),
            "Maiden" => Some(NameUse::Maiden),
            _ => None,
        });
        HumanName {
            use_type,
            family: n.family.clone(),
            given: n.given.clone(),
            prefix: n.prefix.clone(),
            suffix: n.suffix.clone(),
        }
    }

    /// Rebuild a domain [`Identifier`] from a stored `person_identifiers`
    /// row, parsing the stringified `identifier_type` / `use_type`.
    fn identifier_from_db(id: &person_identifiers::Model) -> Identifier {
        use crate::models::{IdentifierType, IdentifierUse};
        let identifier_type = match id.identifier_type.as_str() {
            "MRN" => IdentifierType::MRN,
            "SSN" => IdentifierType::SSN,
            "DL" => IdentifierType::DL,
            "NPI" => IdentifierType::NPI,
            "PPN" => IdentifierType::PPN,
            "TAX" => IdentifierType::TAX,
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
    }

    /// Reassemble a domain [`Person`] from its parent row and child rows.
    ///
    /// Parses stringified enums back to their domain variants (unknown
    /// values fall back to `Other`/`Unknown` or are dropped). Errors if no
    /// primary name is present. Note `tax_id`, `documents`, and
    /// `emergency_contacts` are not yet persisted and come back empty.
    fn from_db_models(
        db_person: persons::Model,
        db_names: &[person_names::Model],
        db_identifiers: &[person_identifiers::Model],
        db_addresses: &[person_addresses::Model],
        db_contacts: &[person_contacts::Model],
        db_links: &[person_links::Model],
    ) -> Result<Person> {
        use crate::models::{ContactPointSystem, ContactPointUse, Gender, LinkType};

        // Parse gender. DB stores lowercase per CHECK constraint
        // ('male'/'female'/'other'/'unknown'); accept PascalCase too
        // so rows written by older code (when persisted via the
        // legacy `format!("{:?}", …)` path) still round-trip.
        let gender = match db_person.gender.to_lowercase().as_str() {
            "male" => Gender::Male,
            "female" => Gender::Female,
            "other" => Gender::Other,
            _ => Gender::Unknown,
        };

        // Get primary name
        let primary_name = db_names
            .iter()
            .find(|n| n.is_primary)
            .ok_or_else(|| crate::Error::Validation("Person has no primary name".to_string()))?;
        let name = Self::human_name_from_db(primary_name);

        // Additional names
        let additional_names = db_names
            .iter()
            .filter(|n| !n.is_primary)
            .map(Self::human_name_from_db)
            .collect();

        // Identifiers
        let identifiers = db_identifiers
            .iter()
            .map(Self::identifier_from_db)
            .collect();

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

        // Telecom
        let telecom = db_contacts
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
            .collect();

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

                Some(PersonLink {
                    other_person_id: link.other_person_id,
                    link_type,
                })
            })
            .collect();

        Ok(Person {
            id: db_person.id,
            identifiers,
            active: db_person.active,
            name,
            additional_names,
            telecom,
            gender,
            birth_date: db_person.birth_date.map(time_to_date),
            deceased: db_person.deceased,
            deceased_datetime: db_person.deceased_datetime.map(offset_to_ts),
            addresses,
            marital_status: db_person.marital_status,
            multiple_birth: db_person.multiple_birth,
            tax_id: db_person.tax_id,
            // Loaded separately from child tables by `load_extra_collections`.
            documents: vec![],
            emergency_contacts: vec![],
            photo: vec![],
            managing_organization: db_person.managing_organization_id,
            links,
            created_at: offset_to_ts(db_person.created_at),
            updated_at: offset_to_ts(db_person.updated_at),
        })
    }

    /// Fetch every child row (names/identifiers/addresses/contacts/links)
    /// for one person, in a fixed tuple order.
    async fn load_associations(
        &self,
        person_id: &Uuid,
    ) -> Result<(
        Vec<person_names::Model>,
        Vec<person_identifiers::Model>,
        Vec<person_addresses::Model>,
        Vec<person_contacts::Model>,
        Vec<person_links::Model>,
    )> {
        let db_names = person_names::Entity::find()
            .filter(person_names::Column::PersonId.eq(*person_id))
            .all(&self.db)
            .await?;

        let db_identifiers = person_identifiers::Entity::find()
            .filter(person_identifiers::Column::PersonId.eq(*person_id))
            .all(&self.db)
            .await?;

        let db_addresses = person_addresses::Entity::find()
            .filter(person_addresses::Column::PersonId.eq(*person_id))
            .all(&self.db)
            .await?;

        let db_contacts = person_contacts::Entity::find()
            .filter(person_contacts::Column::PersonId.eq(*person_id))
            .all(&self.db)
            .await?;

        let db_links = person_links::Entity::find()
            .filter(person_links::Column::PersonId.eq(*person_id))
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
    /// for `person.id` and populate them onto `person`.
    async fn load_extra_collections(&self, person: &mut Person) -> Result<()> {
        let id = person.id;

        let doc_rows = person_documents::Entity::find()
            .filter(person_documents::Column::PersonId.eq(id))
            .order_by_asc(person_documents::Column::Position)
            .all(&self.db)
            .await?;
        person.documents = doc_rows
            .into_iter()
            .map(|r| IdentityDocument {
                document_type: tag_to_enum(Some(&r.document_type)).unwrap_or(DocumentType::Other),
                number: r.number,
                issuing_country: r.issuing_country,
                issuing_authority: r.issuing_authority,
                issue_date: r.issue_date.map(time_to_date),
                expiry_date: r.expiry_date.map(time_to_date),
                verified: r.verified,
            })
            .collect();

        let ec_rows = person_emergency_contacts::Entity::find()
            .filter(person_emergency_contacts::Column::PersonId.eq(id))
            .order_by_asc(person_emergency_contacts::Column::Position)
            .all(&self.db)
            .await?;
        let mut emergency_contacts = Vec::with_capacity(ec_rows.len());
        for ec in ec_rows {
            let tel_rows = person_emergency_contact_telecom::Entity::find()
                .filter(person_emergency_contact_telecom::Column::EmergencyContactId.eq(ec.id))
                .order_by_asc(person_emergency_contact_telecom::Column::Position)
                .all(&self.db)
                .await?;
            let telecom = tel_rows
                .into_iter()
                .map(|t| ContactPoint {
                    system: tag_to_enum(Some(&t.system)).unwrap_or(ContactPointSystem::Other),
                    value: t.value,
                    use_type: tag_to_enum(t.use_type.as_ref()),
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
                use_type: tag_to_enum(ec.address_use_type.as_ref()),
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
        person.emergency_contacts = emergency_contacts;

        let photo_rows = person_photos::Entity::find()
            .filter(person_photos::Column::PersonId.eq(id))
            .order_by_asc(person_photos::Column::Position)
            .all(&self.db)
            .await?;
        person.photo = photo_rows.into_iter().map(|r| r.url).collect();

        Ok(())
    }
}

#[async_trait::async_trait]
impl PersonRepository for SeaOrmPersonRepository {
    /// Insert the person and all child rows in one transaction, then
    /// reload, publish a `Created` event, and write a CREATE audit row.
    async fn create(&self, person: &Person) -> Result<Person> {
        let txn = self.db.begin().await?;

        let (new_person, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            Self::to_active_models(person);

        // Insert person
        let db_person = new_person.insert(&txn).await?;

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
        insert_extra_collections(&txn, person).await?;

        // Durable event bus (Phase 2): under the outbox transport, write
        // the `event_outbox` row **inside this same transaction**, before
        // the commit, so the entity rows and the event commit atomically
        // (or roll back together). A no-op under the memory transport,
        // which keeps the post-commit in-memory publish below.
        self.enqueue_outbox(&txn, person, EventKind::Created).await?;

        txn.commit().await?;

        // Load associations
        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(&db_person.id).await?;

        let mut result = Self::from_db_models(
            db_person,
            &db_names,
            &db_identifiers,
            &db_addresses,
            &db_contacts,
            &db_links,
        )?;
        self.load_extra_collections(&mut result).await?;

        // Publish event
        self.publish_event(crate::streaming::PersonEvent::Created {
            person: result.clone(),
            timestamp: Utc::now(),
        });

        // Log audit
        if let Ok(person_json) = serde_json::to_value(&result) {
            self.log_audit(
                "CREATE",
                result.id,
                None,
                Some(person_json),
                &AuditContext::default(),
            )
            .await;
        }

        Ok(result)
    }

    /// Load a person and its associations, skipping soft-deleted rows.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Person>> {
        let db_person = persons::Entity::find_by_id(*id)
            .filter(persons::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;

        let Some(db_person) = db_person else {
            return Ok(None);
        };

        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(id).await?;

        let mut person = Self::from_db_models(
            db_person,
            &db_names,
            &db_identifiers,
            &db_addresses,
            &db_contacts,
            &db_links,
        )?;
        self.load_extra_collections(&mut person).await?;
        Ok(Some(person))
    }

    /// Update the parent row, then delete-and-reinsert all child rows in
    /// one transaction (a simple full-replace), publish an `Updated`
    /// event, and write an UPDATE audit row with the before/after JSON.
    async fn update(&self, person: &Person) -> Result<Person> {
        // Get old values for audit
        let old_person = self.get_by_id(&person.id).await?;

        let txn = self.db.begin().await?;

        // Parent-row update + wholesale child-row replacement, shared with
        // `merge` (see `apply_update_rows`).
        apply_update_rows(&txn, person).await?;

        // Outbox row shares the update transaction (see `create`).
        self.enqueue_outbox(&txn, person, EventKind::Updated).await?;

        txn.commit().await?;

        // Fetch and return updated person
        let result = self
            .get_by_id(&person.id)
            .await?
            .ok_or_else(|| crate::Error::Validation("Person not found after update".to_string()))?;

        // Publish event
        self.publish_event(crate::streaming::PersonEvent::Updated {
            person: result.clone(),
            timestamp: Utc::now(),
        });

        // Log audit
        if let Some(old_json) = old_person
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

    /// Merge `duplicate_id` into `survivor` atomically: apply the
    /// survivor's parent + child updates and soft-delete the duplicate in
    /// one transaction. Under the outbox transport, a `Merged` outbox row
    /// for the survivor (carrying the duplicate's pid via `merged_from`)
    /// and a `Deleted` outbox row for the duplicate are enqueued on the
    /// same transaction, so the whole merge and both events commit (or
    /// roll back) together. Post-commit it publishes the in-memory
    /// `Merged` + `Deleted` events and writes the UPDATE/DELETE audit rows.
    async fn merge(&self, survivor: &Person, duplicate_id: &Uuid) -> Result<Person> {
        // Snapshots for the audit trail: the survivor's pre-image for the
        // UPDATE row, the duplicate's pre-image for the DELETE row (and to
        // build the duplicate's `Deleted` outbox envelope).
        let old_survivor = self.get_by_id(&survivor.id).await?;
        let old_duplicate = self.get_by_id(duplicate_id).await?;

        // The whole merge — survivor row updates + duplicate soft-delete +
        // both outbox rows — commits (or rolls back) under one transaction.
        let txn = self.db.begin().await?;

        // Apply the survivor's parent + child rows (shared with `update`).
        apply_update_rows(&txn, survivor).await?;

        // Soft-delete the duplicate (shared with `delete`).
        apply_soft_delete_row(&txn, duplicate_id).await?;

        // Durable event bus (Phase 2): a `Merged` outbox row for the
        // survivor (carrying the duplicate's pid via `merged_from`, so a
        // merge-repointing consumer can move edges off the duplicate) and a
        // `Deleted` outbox row for the duplicate — both inside this
        // transaction. A no-op under the memory transport.
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
            .ok_or_else(|| crate::Error::Validation("Person not found after merge".to_string()))?;

        // Post-commit in-memory stream (memory transport keeps this): a
        // `Merged` for the survivor and a `Deleted` for the duplicate.
        self.publish_event(crate::streaming::PersonEvent::Merged {
            source_id: *duplicate_id,
            target_id: result.id,
            timestamp: Utc::now(),
        });
        self.publish_event(crate::streaming::PersonEvent::Deleted {
            person_id: *duplicate_id,
            timestamp: Utc::now(),
        });

        // Audit rows: an UPDATE trail for the survivor and a DELETE trail
        // for the duplicate.
        if let Some(old_json) = old_survivor.as_ref().and_then(|p| serde_json::to_value(p).ok())
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

    /// Soft-delete by stamping `deleted_at`/`deleted_by`; child rows are
    /// retained. Publishes a `Deleted` event and writes a DELETE audit row.
    async fn delete(&self, id: &Uuid) -> Result<()> {
        // Get old values for audit
        let old_person = self.get_by_id(id).await?;

        // Soft delete: stamp `deleted_at`/`deleted_by` only. Unlike
        // create/update, this is a single row update with no existing
        // transaction. Under the outbox transport we open one here so the
        // tombstone write and the `deleted` outbox row commit atomically;
        // the memory transport keeps the plain, tx-free update.
        if self.transport.is_outbox() {
            let txn = self.db.begin().await?;
            apply_soft_delete_row(&txn, id).await?;
            if let Some(old) = old_person.as_ref() {
                self.enqueue_outbox(&txn, old, EventKind::Deleted).await?;
            }
            txn.commit().await?;
        } else {
            apply_soft_delete_row(&self.db, id).await?;
        }

        // Publish event
        self.publish_event(crate::streaming::PersonEvent::Deleted {
            person_id: *id,
            timestamp: Utc::now(),
        });

        // Log audit
        if let Some(old_person) = old_person
            && let Ok(old_json) = serde_json::to_value(&old_person)
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

    /// SQL `LIKE` search over lowercased family name; resolves each
    /// matched person id to a full record. (Tantivy is the richer path.)
    async fn search(&self, query: &str) -> Result<Vec<Person>> {
        let search_pattern = format!("%{}%", query.to_lowercase());

        let person_ids: Vec<Uuid> = person_names::Entity::find()
            .filter(Expr::cust_with_values(
                "LOWER(family) LIKE $1",
                [search_pattern],
            ))
            .select_only()
            .column(person_names::Column::PersonId)
            .distinct()
            .into_tuple()
            .all(&self.db)
            .await?;

        let mut persons = Vec::new();
        for person_id in person_ids {
            if let Some(person) = self.get_by_id(&person_id).await? {
                persons.push(person);
            }
        }

        Ok(persons)
    }

    /// Page through active, non-deleted persons, hydrating each to a full
    /// record. (One follow-up `get_by_id` per row — fine for modest pages.)
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Person>> {
        let db_persons: Vec<persons::Model> = persons::Entity::find()
            .filter(persons::Column::DeletedAt.is_null())
            .filter(persons::Column::Active.eq(true))
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await?;

        let mut persons = Vec::new();
        for db_person in db_persons {
            if let Some(person) = self.get_by_id(&db_person.id).await? {
                persons.push(person);
            }
        }

        Ok(persons)
    }
}

/// DB-gated (`#[ignore]`) atomicity tests for the outbox write path.
/// They require a migrated `PostgreSQL` via `DATABASE_URL` and are skipped
/// by a bare `cargo test`; run with
/// `DATABASE_URL=… cargo test --lib -- --ignored`. They must COMPILE
/// under a bare `cargo test --lib`.
#[cfg(test)]
mod tests {
    use super::{PersonRepository, SeaOrmPersonRepository};
    use crate::db::models::event_outbox;
    use crate::models::{Gender, HumanName, Person};
    use crate::streaming::EventTransport;
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    async fn connect() -> sea_orm::DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    fn a_person(family: &str) -> Person {
        Person::new(
            HumanName {
                use_type: None,
                family: family.to_string(),
                given: vec!["Test".to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            Gender::Unknown,
        )
    }

    /// Create writes a `created` outbox row in the same transaction as the
    /// entity rows.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn create_enqueues_a_created_outbox_row() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let person = repo.create(&a_person("Created")).await.unwrap();

        let rows = event_outbox::Entity::find()
            .filter(event_outbox::Column::EntityPid.eq(person.id))
            .filter(event_outbox::Column::Kind.eq("created"))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1, "one created outbox row for the new person");
        assert_eq!(rows[0].entity, "person");
    }

    /// Merge writes, in one transaction: a `merged` outbox row for the
    /// survivor carrying the duplicate's pid in `merged_from`, plus a
    /// `deleted` outbox row for the duplicate.
    #[tokio::test]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn merge_enqueues_merged_with_merged_from_and_deleted() {
        let db = connect().await;
        let repo = SeaOrmPersonRepository::new(db.clone()).with_transport(EventTransport::Outbox);

        let survivor = repo.create(&a_person("Survivor")).await.unwrap();
        let duplicate = repo.create(&a_person("Duplicate")).await.unwrap();

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
        assert_eq!(deleted_rows.len(), 1, "one deleted outbox row for duplicate");
    }
}
