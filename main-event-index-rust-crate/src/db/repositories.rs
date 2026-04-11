//! Repository pattern implementations for database operations

use sea_orm::*;
use sea_orm::sea_query::Expr;
use chrono::Utc;
use uuid::Uuid;

use crate::models::{Event, HumanName, Address, ContactPoint, Identifier, EventLink};
use crate::Result;
use super::models::*;

/// Audit context for tracking user actions
#[derive(Debug, Clone)]
pub struct AuditContext {
    pub user_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl Default for AuditContext {
    fn default() -> Self {
        Self {
            user_id: Some("system".to_string()),
            ip_address: None,
            user_agent: None,
        }
    }
}

/// Event repository trait
#[async_trait::async_trait]
pub trait EventRepository: Send + Sync {
    /// Create a new event
    async fn create(&self, event: &Event) -> Result<Event>;

    /// Get a event by ID
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Event>>;

    /// Update a event
    async fn update(&self, event: &Event) -> Result<Event>;

    /// Delete a event (soft delete)
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// Search events by name
    async fn search(&self, query: &str) -> Result<Vec<Event>>;

    /// List all active events (non-deleted)
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Event>>;
}

/// SeaORM-based event repository implementation
pub struct SeaOrmEventRepository {
    db: DatabaseConnection,
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
}

impl SeaOrmEventRepository {
    /// Create a new repository with the given database connection
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            event_publisher: None,
            audit_log: None,
        }
    }

    /// Set the event publisher for this repository
    pub fn with_event_publisher(
        mut self,
        publisher: std::sync::Arc<dyn crate::streaming::EventProducer>,
    ) -> Self {
        self.event_publisher = Some(publisher);
        self
    }

    /// Set the audit log repository
    pub fn with_audit_log(
        mut self,
        audit_log: std::sync::Arc<super::audit::AuditLogRepository>,
    ) -> Self {
        self.audit_log = Some(audit_log);
        self
    }

    /// Publish an event if publisher is configured
    fn publish_event(&self, event: crate::streaming::EventEvent) {
        if let Some(ref publisher) = self.event_publisher {
            if let Err(e) = publisher.publish(event) {
                tracing::error!("Failed to publish event: {}", e);
            }
        }
    }

    /// Log to audit trail if configured
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
                    "Event",
                    entity_id,
                    new_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                "UPDATE" => audit_log.log_update(
                    "Event",
                    entity_id,
                    old_values.unwrap_or(serde_json::Value::Null),
                    new_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                "DELETE" => audit_log.log_delete(
                    "Event",
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

    /// Convert domain Event model to SeaORM active models
    fn to_active_models(&self, event: &Event) -> (
        events::ActiveModel,
        Vec<event_names::ActiveModel>,
        Vec<event_identifiers::ActiveModel>,
        Vec<event_addresses::ActiveModel>,
        Vec<event_contacts::ActiveModel>,
        Vec<event_links::ActiveModel>,
    ) {
        let new_event = events::ActiveModel {
            id: Set(event.id),
            active: Set(event.active),
            gender: Set(format!("{:?}", event.gender)),
            birth_date: Set(event.birth_date),
            deceased: Set(event.deceased),
            deceased_datetime: Set(event.deceased_datetime),
            marital_status: Set(event.marital_status.clone()),
            multiple_birth: Set(event.multiple_birth),
            managing_organization_id: Set(event.managing_organization),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            created_by: Set(None),
            updated_by: Set(None),
            deleted_at: Set(None),
            deleted_by: Set(None),
        };

        // Primary name
        let mut names = vec![event_names::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            use_type: Set(event.name.use_type.as_ref().map(|u| format!("{:?}", u))),
            family: Set(event.name.family.clone()),
            given: Set(event.name.given.clone()),
            prefix: Set(event.name.prefix.clone()),
            suffix: Set(event.name.suffix.clone()),
            is_primary: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }];

        // Additional names
        for add_name in &event.additional_names {
            names.push(event_names::ActiveModel {
                id: Set(Uuid::new_v4()),
                event_id: Set(event.id),
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
        let identifiers = event.identifiers.iter().map(|id| event_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            use_type: Set(id.use_type.as_ref().map(|u| format!("{:?}", u))),
            identifier_type: Set(format!("{:?}", id.identifier_type)),
            system: Set(id.system.clone()),
            value: Set(id.value.clone()),
            assigner: Set(id.assigner.clone()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Addresses
        let addresses = event.addresses.iter().enumerate().map(|(idx, addr)| event_addresses::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
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
        let contacts = event.telecom.iter().enumerate().map(|(idx, cp)| event_contacts::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            system: Set(format!("{:?}", cp.system)),
            value: Set(cp.value.clone()),
            use_type: Set(cp.use_type.as_ref().map(|u| format!("{:?}", u))),
            is_primary: Set(idx == 0),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Links
        let links = event.links.iter().map(|link| event_links::ActiveModel {
            id: Set(Uuid::new_v4()),
            event_id: Set(event.id),
            other_event_id: Set(link.other_event_id),
            link_type: Set(format!("{:?}", link.link_type)),
            created_at: Set(Utc::now()),
            created_by: Set(None),
        }).collect();

        (new_event, names, identifiers, addresses, contacts, links)
    }

    /// Convert database models to domain Event model
    fn from_db_models(
        &self,
        db_event: events::Model,
        db_names: Vec<event_names::Model>,
        db_identifiers: Vec<event_identifiers::Model>,
        db_addresses: Vec<event_addresses::Model>,
        db_contacts: Vec<event_contacts::Model>,
        db_links: Vec<event_links::Model>,
    ) -> Result<Event> {
        use crate::models::{Gender, NameUse, ContactPointSystem, ContactPointUse, LinkType, IdentifierType, IdentifierUse};

        // Parse gender
        let gender = match db_event.gender.as_str() {
            "Male" => Gender::Male,
            "Female" => Gender::Female,
            "Other" => Gender::Other,
            _ => Gender::Unknown,
        };

        // Get primary name
        let primary_name = db_names.iter()
            .find(|n| n.is_primary)
            .ok_or_else(|| crate::Error::Validation("Event has no primary name".to_string()))?;

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

                Some(EventLink {
                    other_event_id: link.other_event_id,
                    link_type,
                })
            })
            .collect();

        Ok(Event {
            id: db_event.id,
            identifiers,
            active: db_event.active,
            name,
            additional_names,
            telecom,
            gender,
            birth_date: db_event.birth_date,
            deceased: db_event.deceased,
            deceased_datetime: db_event.deceased_datetime,
            addresses,
            marital_status: db_event.marital_status,
            multiple_birth: db_event.multiple_birth,
            tax_id: None, // TODO: Load from DB
            documents: vec![], // TODO: Load from DB
            emergency_contacts: vec![], // TODO: Load from DB
            photo: vec![], // Not stored in DB yet
            managing_organization: db_event.managing_organization_id,
            links,
            created_at: db_event.created_at,
            updated_at: db_event.updated_at,
        })
    }

    /// Load all associated data for a event
    async fn load_associations(&self, event_id: &Uuid) -> Result<(
        Vec<event_names::Model>,
        Vec<event_identifiers::Model>,
        Vec<event_addresses::Model>,
        Vec<event_contacts::Model>,
        Vec<event_links::Model>,
    )> {
        let db_names = event_names::Entity::find()
            .filter(event_names::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;

        let db_identifiers = event_identifiers::Entity::find()
            .filter(event_identifiers::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;

        let db_addresses = event_addresses::Entity::find()
            .filter(event_addresses::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;

        let db_contacts = event_contacts::Entity::find()
            .filter(event_contacts::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;

        let db_links = event_links::Entity::find()
            .filter(event_links::Column::EventId.eq(*event_id))
            .all(&self.db)
            .await?;

        Ok((db_names, db_identifiers, db_addresses, db_contacts, db_links))
    }
}

#[async_trait::async_trait]
impl EventRepository for SeaOrmEventRepository {
    async fn create(&self, event: &Event) -> Result<Event> {
        let txn = self.db.begin().await?;

        let (new_event, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            self.to_active_models(event);

        // Insert event
        let db_event = new_event.insert(&txn).await?;

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
            self.load_associations(&db_event.id).await?;

        let result = self.from_db_models(db_event, db_names, db_identifiers, db_addresses, db_contacts, db_links)?;

        // Publish event
        self.publish_event(crate::streaming::EventEvent::Created {
            event: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Ok(event_json) = serde_json::to_value(&result) {
            self.log_audit("CREATE", result.id, None, Some(event_json), &AuditContext::default()).await;
        }

        Ok(result)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Event>> {
        let db_event = events::Entity::find_by_id(*id)
            .filter(events::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;

        let db_event = match db_event {
            Some(p) => p,
            None => return Ok(None),
        };

        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(id).await?;

        self.from_db_models(db_event, db_names, db_identifiers, db_addresses, db_contacts, db_links)
            .map(Some)
    }

    async fn update(&self, event: &Event) -> Result<Event> {
        // Get old values for audit
        let old_event = self.get_by_id(&event.id).await?;

        let txn = self.db.begin().await?;

        // Update event
        let update_model = events::ActiveModel {
            id: Set(event.id),
            active: Set(event.active),
            gender: Set(format!("{:?}", event.gender)),
            birth_date: Set(event.birth_date),
            deceased: Set(event.deceased),
            deceased_datetime: Set(event.deceased_datetime),
            marital_status: Set(event.marital_status.clone()),
            multiple_birth: Set(event.multiple_birth),
            managing_organization_id: Set(event.managing_organization),
            updated_at: Set(Utc::now()),
            updated_by: Set(None),
            ..Default::default()
        };
        update_model.update(&txn).await?;

        // Delete existing associated data
        event_names::Entity::delete_many()
            .filter(event_names::Column::EventId.eq(event.id))
            .exec(&txn).await?;

        event_identifiers::Entity::delete_many()
            .filter(event_identifiers::Column::EventId.eq(event.id))
            .exec(&txn).await?;

        event_addresses::Entity::delete_many()
            .filter(event_addresses::Column::EventId.eq(event.id))
            .exec(&txn).await?;

        event_contacts::Entity::delete_many()
            .filter(event_contacts::Column::EventId.eq(event.id))
            .exec(&txn).await?;

        event_links::Entity::delete_many()
            .filter(event_links::Column::EventId.eq(event.id))
            .exec(&txn).await?;

        // Re-insert associated data
        let (_, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            self.to_active_models(event);

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

        // Fetch and return updated event
        let result = self.get_by_id(&event.id).await?
            .ok_or_else(|| crate::Error::Validation("Event not found after update".to_string()))?;

        // Publish event
        self.publish_event(crate::streaming::EventEvent::Updated {
            event: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_json) = old_event.as_ref().and_then(|p| serde_json::to_value(p).ok()) {
            if let Ok(new_json) = serde_json::to_value(&result) {
                self.log_audit("UPDATE", result.id, Some(old_json), Some(new_json), &AuditContext::default()).await;
            }
        }

        Ok(result)
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        // Get old values for audit
        let old_event = self.get_by_id(id).await?;

        // Soft delete
        let update_model = events::ActiveModel {
            id: Set(*id),
            deleted_at: Set(Some(Utc::now())),
            deleted_by: Set(Some("system".to_string())),
            ..Default::default()
        };
        update_model.update(&self.db).await?;

        // Publish event
        self.publish_event(crate::streaming::EventEvent::Deleted {
            event_id: *id,
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_event) = old_event {
            if let Ok(old_json) = serde_json::to_value(&old_event) {
                self.log_audit("DELETE", *id, Some(old_json), None, &AuditContext::default()).await;
            }
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Event>> {
        let search_pattern = format!("%{}%", query.to_lowercase());

        let event_ids: Vec<Uuid> = event_names::Entity::find()
            .filter(Expr::cust_with_values("LOWER(family) LIKE $1", [search_pattern]))
            .select_only()
            .column(event_names::Column::EventId)
            .distinct()
            .into_tuple()
            .all(&self.db)
            .await?;

        let mut events = Vec::new();
        for event_id in event_ids {
            if let Some(event) = self.get_by_id(&event_id).await? {
                events.push(event);
            }
        }

        Ok(events)
    }

    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Event>> {
        let db_events: Vec<events::Model> = events::Entity::find()
            .filter(events::Column::DeletedAt.is_null())
            .filter(events::Column::Active.eq(true))
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await?;

        let mut events = Vec::new();
        for db_event in db_events {
            if let Some(event) = self.get_by_id(&db_event.id).await? {
                events.push(event);
            }
        }

        Ok(events)
    }
}
