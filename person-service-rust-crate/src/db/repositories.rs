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

use sea_orm::*;
use sea_orm::sea_query::Expr;
use chrono::Utc;
use uuid::Uuid;

use crate::models::{Person, HumanName, Address, ContactPoint, Identifier, PersonLink};
use crate::Result;
use super::models::*;

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

    /// Soft-delete a person (sets `deleted_at`; row is retained).
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// Find persons whose family name matches `query` (case-insensitive).
    async fn search(&self, query: &str) -> Result<Vec<Person>>;

    /// Page through active, non-deleted persons via `limit`/`offset`.
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Person>>;
}

/// PostgreSQL [`PersonRepository`] backed by SeaORM.
///
/// Optionally fans out to an event publisher and audit log; both are set
/// via the `with_*` builder methods and are no-ops when absent.
pub struct SeaOrmPersonRepository {
    /// The SeaORM connection (cheap to clone, internally pooled).
    db: DatabaseConnection,
    /// Optional sink for [`PersonEvent`](crate::streaming::PersonEvent)s.
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    /// Optional audit-trail writer.
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
}

impl SeaOrmPersonRepository {
    /// Build a repository over `db` with no event/audit sinks attached.
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            event_publisher: None,
            audit_log: None,
        }
    }

    /// Attach an event publisher; returns `self` for chaining.
    pub fn with_event_publisher(
        mut self,
        publisher: std::sync::Arc<dyn crate::streaming::EventProducer>,
    ) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Attach an audit-log repository; returns `self` for chaining.
    pub fn with_audit_log(
        mut self,
        audit_log: std::sync::Arc<super::audit::AuditLogRepository>,
    ) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// Publish `event` if a publisher is set; log and swallow any error.
    fn publish_event(&self, event: crate::streaming::PersonEvent) {
        if let Some(ref publisher) = self.event_publisher {
            if let Err(e) = publisher.publish(event) {
                tracing::error!("Failed to publish event: {}", e);
            }
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
                "CREATE" => audit_log.log_create(
                    "Person",
                    entity_id,
                    new_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                "UPDATE" => audit_log.log_update(
                    "Person",
                    entity_id,
                    old_values.unwrap_or(serde_json::Value::Null),
                    new_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                "DELETE" => audit_log.log_delete(
                    "Person",
                    entity_id,
                    old_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                _ => Ok(()),
            };

            if let Err(e) = result {
                tracing::error!("Failed to log audit: {}", e);
            }
        }
    }

    /// Explode a domain [`Person`] into the SeaORM active models for the
    /// parent row and each child table.
    ///
    /// The primary name gets `is_primary = true`; the first address and
    /// first contact are flagged primary by position. Enum fields are
    /// stringified (`{:?}`) except `gender`, which is lowercased to honor
    /// the DB CHECK constraint.
    fn to_active_models(&self, person: &Person) -> (
        persons::ActiveModel,
        Vec<person_names::ActiveModel>,
        Vec<person_identifiers::ActiveModel>,
        Vec<person_addresses::ActiveModel>,
        Vec<person_contacts::ActiveModel>,
        Vec<person_links::ActiveModel>,
    ) {
        let new_person = persons::ActiveModel {
            id: Set(person.id),
            active: Set(person.active),
            // DB CHECK constraint enforces lowercase ('male'/'female'/'other'/'unknown');
            // Gender's serde rename_all="lowercase" produces the same shape.
            gender: Set(format!("{:?}", person.gender).to_lowercase()),
            birth_date: Set(person.birth_date),
            deceased: Set(person.deceased),
            deceased_datetime: Set(person.deceased_datetime),
            marital_status: Set(person.marital_status.clone()),
            multiple_birth: Set(person.multiple_birth),
            managing_organization_id: Set(person.managing_organization),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            created_by: Set(None),
            updated_by: Set(None),
            deleted_at: Set(None),
            deleted_by: Set(None),
        };

        // Primary name
        let mut names = vec![person_names::ActiveModel {
            id: Set(Uuid::new_v4()),
            person_id: Set(person.id),
            use_type: Set(person.name.use_type.as_ref().map(|u| format!("{:?}", u))),
            family: Set(person.name.family.clone()),
            given: Set(person.name.given.clone()),
            prefix: Set(person.name.prefix.clone()),
            suffix: Set(person.name.suffix.clone()),
            is_primary: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }];

        // Additional names
        for add_name in &person.additional_names {
            names.push(person_names::ActiveModel {
                id: Set(Uuid::new_v4()),
                person_id: Set(person.id),
                use_type: Set(add_name.use_type.as_ref().map(|u| format!("{:?}", u))),
                family: Set(add_name.family.clone()),
                given: Set(add_name.given.clone()),
                prefix: Set(add_name.prefix.clone()),
                suffix: Set(add_name.suffix.clone()),
                is_primary: Set(false),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            });
        }

        // Identifiers
        let identifiers = person.identifiers.iter().map(|id| person_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            person_id: Set(person.id),
            use_type: Set(id.use_type.as_ref().map(|u| format!("{:?}", u))),
            identifier_type: Set(format!("{:?}", id.identifier_type)),
            system: Set(id.system.clone()),
            value: Set(id.value.clone()),
            assigner: Set(id.assigner.clone()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Addresses
        let addresses = person.addresses.iter().enumerate().map(|(idx, addr)| person_addresses::ActiveModel {
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
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Contacts
        let contacts = person.telecom.iter().enumerate().map(|(idx, cp)| person_contacts::ActiveModel {
            id: Set(Uuid::new_v4()),
            person_id: Set(person.id),
            system: Set(format!("{:?}", cp.system)),
            value: Set(cp.value.clone()),
            use_type: Set(cp.use_type.as_ref().map(|u| format!("{:?}", u))),
            is_primary: Set(idx == 0),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Links
        let links = person.links.iter().map(|link| person_links::ActiveModel {
            id: Set(Uuid::new_v4()),
            person_id: Set(person.id),
            other_person_id: Set(link.other_person_id),
            link_type: Set(format!("{:?}", link.link_type)),
            created_at: Set(Utc::now()),
            created_by: Set(None),
        }).collect();

        (new_person, names, identifiers, addresses, contacts, links)
    }

    /// Reassemble a domain [`Person`] from its parent row and child rows.
    ///
    /// Parses stringified enums back to their domain variants (unknown
    /// values fall back to `Other`/`Unknown` or are dropped). Errors if no
    /// primary name is present. Note `tax_id`, `documents`, and
    /// `emergency_contacts` are not yet persisted and come back empty.
    fn from_db_models(
        &self,
        db_person: persons::Model,
        db_names: Vec<person_names::Model>,
        db_identifiers: Vec<person_identifiers::Model>,
        db_addresses: Vec<person_addresses::Model>,
        db_contacts: Vec<person_contacts::Model>,
        db_links: Vec<person_links::Model>,
    ) -> Result<Person> {
        use crate::models::{Gender, NameUse, ContactPointSystem, ContactPointUse, LinkType, IdentifierType, IdentifierUse};

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
        let primary_name = db_names.iter()
            .find(|n| n.is_primary)
            .ok_or_else(|| crate::Error::Validation("Person has no primary name".to_string()))?;

        let name = HumanName {
            use_type: primary_name.use_type.as_ref().and_then(|u| match u.as_str() {
                "Usual" => Some(NameUse::Usual),
                "Official" => Some(NameUse::Official),
                "Temp" => Some(NameUse::Temp),
                "Nickname" => Some(NameUse::Nickname),
                "Anonymous" => Some(NameUse::Anonymous),
                "Old" => Some(NameUse::Old),
                "Maiden" => Some(NameUse::Maiden),
                _ => None,
            }),
            family: primary_name.family.clone(),
            given: primary_name.given.clone(),
            prefix: primary_name.prefix.clone(),
            suffix: primary_name.suffix.clone(),
        };

        // Additional names
        let additional_names = db_names.iter()
            .filter(|n| !n.is_primary)
            .map(|n| HumanName {
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
            })
            .collect();

        // Identifiers
        let identifiers = db_identifiers.iter()
            .map(|id| {
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
            })
            .collect();

        // Addresses
        let addresses = db_addresses.iter()
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
        let telecom = db_contacts.iter()
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
        let links = db_links.iter()
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
            birth_date: db_person.birth_date,
            deceased: db_person.deceased,
            deceased_datetime: db_person.deceased_datetime,
            addresses,
            marital_status: db_person.marital_status,
            multiple_birth: db_person.multiple_birth,
            tax_id: None, // TODO: Load from DB
            documents: vec![], // TODO: Load from DB
            emergency_contacts: vec![], // TODO: Load from DB
            photo: vec![], // Not stored in DB yet
            managing_organization: db_person.managing_organization_id,
            links,
            created_at: db_person.created_at,
            updated_at: db_person.updated_at,
        })
    }

    /// Fetch every child row (names/identifiers/addresses/contacts/links)
    /// for one person, in a fixed tuple order.
    async fn load_associations(&self, person_id: &Uuid) -> Result<(
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

        Ok((db_names, db_identifiers, db_addresses, db_contacts, db_links))
    }
}

#[async_trait::async_trait]
impl PersonRepository for SeaOrmPersonRepository {
    /// Insert the person and all child rows in one transaction, then
    /// reload, publish a `Created` event, and write a CREATE audit row.
    async fn create(&self, person: &Person) -> Result<Person> {
        let txn = self.db.begin().await?;

        let (new_person, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            self.to_active_models(person);

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

        txn.commit().await?;

        // Load associations
        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(&db_person.id).await?;

        let result = self.from_db_models(db_person, db_names, db_identifiers, db_addresses, db_contacts, db_links)?;

        // Publish event
        self.publish_event(crate::streaming::PersonEvent::Created {
            person: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Ok(person_json) = serde_json::to_value(&result) {
            self.log_audit("CREATE", result.id, None, Some(person_json), &AuditContext::default()).await;
        }

        Ok(result)
    }

    /// Load a person and its associations, skipping soft-deleted rows.
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Person>> {
        let db_person = persons::Entity::find_by_id(*id)
            .filter(persons::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;

        let db_person = match db_person {
            Some(p) => p,
            None => return Ok(None),
        };

        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(id).await?;

        self.from_db_models(db_person, db_names, db_identifiers, db_addresses, db_contacts, db_links)
            .map(Some)
    }

    /// Update the parent row, then delete-and-reinsert all child rows in
    /// one transaction (a simple full-replace), publish an `Updated`
    /// event, and write an UPDATE audit row with the before/after JSON.
    async fn update(&self, person: &Person) -> Result<Person> {
        // Get old values for audit
        let old_person = self.get_by_id(&person.id).await?;

        let txn = self.db.begin().await?;

        // Update person
        let update_model = persons::ActiveModel {
            id: Set(person.id),
            active: Set(person.active),
            // DB CHECK constraint enforces lowercase ('male'/'female'/'other'/'unknown');
            // Gender's serde rename_all="lowercase" produces the same shape.
            gender: Set(format!("{:?}", person.gender).to_lowercase()),
            birth_date: Set(person.birth_date),
            deceased: Set(person.deceased),
            deceased_datetime: Set(person.deceased_datetime),
            marital_status: Set(person.marital_status.clone()),
            multiple_birth: Set(person.multiple_birth),
            managing_organization_id: Set(person.managing_organization),
            updated_at: Set(Utc::now()),
            updated_by: Set(None),
            ..Default::default()
        };
        update_model.update(&txn).await?;

        // Delete existing associated data
        person_names::Entity::delete_many()
            .filter(person_names::Column::PersonId.eq(person.id))
            .exec(&txn).await?;

        person_identifiers::Entity::delete_many()
            .filter(person_identifiers::Column::PersonId.eq(person.id))
            .exec(&txn).await?;

        person_addresses::Entity::delete_many()
            .filter(person_addresses::Column::PersonId.eq(person.id))
            .exec(&txn).await?;

        person_contacts::Entity::delete_many()
            .filter(person_contacts::Column::PersonId.eq(person.id))
            .exec(&txn).await?;

        person_links::Entity::delete_many()
            .filter(person_links::Column::PersonId.eq(person.id))
            .exec(&txn).await?;

        // Re-insert associated data
        let (_, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            self.to_active_models(person);

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

        txn.commit().await?;

        // Fetch and return updated person
        let result = self.get_by_id(&person.id).await?
            .ok_or_else(|| crate::Error::Validation("Person not found after update".to_string()))?;

        // Publish event
        self.publish_event(crate::streaming::PersonEvent::Updated {
            person: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_json) = old_person.as_ref().and_then(|p| serde_json::to_value(p).ok()) {
            if let Ok(new_json) = serde_json::to_value(&result) {
                self.log_audit("UPDATE", result.id, Some(old_json), Some(new_json), &AuditContext::default()).await;
            }
        }

        Ok(result)
    }

    /// Soft-delete by stamping `deleted_at`/`deleted_by`; child rows are
    /// retained. Publishes a `Deleted` event and writes a DELETE audit row.
    async fn delete(&self, id: &Uuid) -> Result<()> {
        // Get old values for audit
        let old_person = self.get_by_id(id).await?;

        // Soft delete
        let update_model = persons::ActiveModel {
            id: Set(*id),
            deleted_at: Set(Some(Utc::now())),
            deleted_by: Set(Some("system".to_string())),
            ..Default::default()
        };
        update_model.update(&self.db).await?;

        // Publish event
        self.publish_event(crate::streaming::PersonEvent::Deleted {
            person_id: *id,
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_person) = old_person {
            if let Ok(old_json) = serde_json::to_value(&old_person) {
                self.log_audit("DELETE", *id, Some(old_json), None, &AuditContext::default()).await;
            }
        }

        Ok(())
    }

    /// SQL `LIKE` search over lowercased family name; resolves each
    /// matched person id to a full record. (Tantivy is the richer path.)
    async fn search(&self, query: &str) -> Result<Vec<Person>> {
        let search_pattern = format!("%{}%", query.to_lowercase());

        let person_ids: Vec<Uuid> = person_names::Entity::find()
            .filter(Expr::cust_with_values("LOWER(family) LIKE $1", [search_pattern]))
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
