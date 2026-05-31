//! Repository pattern implementations for database operations

use sea_orm::*;
use sea_orm::sea_query::Expr;
use chrono::Utc;
use uuid::Uuid;

use crate::models::{Worker, HumanName, Address, ContactPoint, Identifier, WorkerLink, WorkerType};
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

/// Worker repository trait
#[async_trait::async_trait]
pub trait WorkerRepository: Send + Sync {
    /// Create a new worker
    async fn create(&self, worker: &Worker) -> Result<Worker>;

    /// Get a worker by ID
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Worker>>;

    /// Update a worker
    async fn update(&self, worker: &Worker) -> Result<Worker>;

    /// Delete a worker (soft delete)
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// Search workers by name
    async fn search(&self, query: &str) -> Result<Vec<Worker>>;

    /// List all active workers (non-deleted)
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Worker>>;
}

/// SeaORM-based worker repository implementation
pub struct SeaOrmWorkerRepository {
    db: DatabaseConnection,
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
}

impl SeaOrmWorkerRepository {
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
    fn publish_event(&self, event: crate::streaming::WorkerEvent) {
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
                    "Worker",
                    entity_id,
                    new_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                "UPDATE" => audit_log.log_update(
                    "Worker",
                    entity_id,
                    old_values.unwrap_or(serde_json::Value::Null),
                    new_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                "DELETE" => audit_log.log_delete(
                    "Worker",
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

    /// Convert domain Worker model to SeaORM active models
    fn to_active_models(&self, worker: &Worker) -> (
        workers::ActiveModel,
        Vec<worker_names::ActiveModel>,
        Vec<worker_identifiers::ActiveModel>,
        Vec<worker_addresses::ActiveModel>,
        Vec<worker_contacts::ActiveModel>,
        Vec<worker_links::ActiveModel>,
    ) {
        let new_worker = workers::ActiveModel {
            id: Set(worker.id),
            active: Set(worker.active),
            worker_type: Set(worker.worker_type.as_ref().map(|wt| wt.to_string())),
            gender: Set(format!("{:?}", worker.gender)),
            birth_date: Set(worker.birth_date),
            deceased: Set(worker.deceased),
            deceased_datetime: Set(worker.deceased_datetime),
            marital_status: Set(worker.marital_status.clone()),
            multiple_birth: Set(worker.multiple_birth),
            managing_organization_id: Set(worker.managing_organization),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            created_by: Set(None),
            updated_by: Set(None),
            deleted_at: Set(None),
            deleted_by: Set(None),
        };

        // Primary name
        let mut names = vec![worker_names::ActiveModel {
            id: Set(Uuid::new_v4()),
            worker_id: Set(worker.id),
            use_type: Set(worker.name.use_type.as_ref().map(|u| format!("{:?}", u))),
            family: Set(worker.name.family.clone()),
            given: Set(worker.name.given.clone()),
            prefix: Set(worker.name.prefix.clone()),
            suffix: Set(worker.name.suffix.clone()),
            is_primary: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }];

        // Additional names
        for add_name in &worker.additional_names {
            names.push(worker_names::ActiveModel {
                id: Set(Uuid::new_v4()),
                worker_id: Set(worker.id),
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
        let identifiers = worker.identifiers.iter().map(|id| worker_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            worker_id: Set(worker.id),
            use_type: Set(id.use_type.as_ref().map(|u| format!("{:?}", u))),
            identifier_type: Set(format!("{:?}", id.identifier_type)),
            system: Set(id.system.clone()),
            value: Set(id.value.clone()),
            assigner: Set(id.assigner.clone()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Addresses
        let addresses = worker.addresses.iter().enumerate().map(|(idx, addr)| worker_addresses::ActiveModel {
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
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Contacts
        let contacts = worker.telecom.iter().enumerate().map(|(idx, cp)| worker_contacts::ActiveModel {
            id: Set(Uuid::new_v4()),
            worker_id: Set(worker.id),
            system: Set(format!("{:?}", cp.system)),
            value: Set(cp.value.clone()),
            use_type: Set(cp.use_type.as_ref().map(|u| format!("{:?}", u))),
            is_primary: Set(idx == 0),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Links
        let links = worker.links.iter().map(|link| worker_links::ActiveModel {
            id: Set(Uuid::new_v4()),
            worker_id: Set(worker.id),
            other_worker_id: Set(link.other_worker_id),
            link_type: Set(format!("{:?}", link.link_type)),
            created_at: Set(Utc::now()),
            created_by: Set(None),
        }).collect();

        (new_worker, names, identifiers, addresses, contacts, links)
    }

    /// Convert database models to domain Worker model
    fn from_db_models(
        &self,
        db_worker: workers::Model,
        db_names: Vec<worker_names::Model>,
        db_identifiers: Vec<worker_identifiers::Model>,
        db_addresses: Vec<worker_addresses::Model>,
        db_contacts: Vec<worker_contacts::Model>,
        db_links: Vec<worker_links::Model>,
    ) -> Result<Worker> {
        use crate::models::{Gender, NameUse, ContactPointSystem, ContactPointUse, LinkType, IdentifierType, IdentifierUse};

        // Parse gender
        let gender = match db_worker.gender.as_str() {
            "Male" => Gender::Male,
            "Female" => Gender::Female,
            "Other" => Gender::Other,
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
        let primary_name = db_names.iter()
            .find(|n| n.is_primary)
            .ok_or_else(|| crate::Error::Validation("Worker has no primary name".to_string()))?;

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
            birth_date: db_worker.birth_date,
            deceased: db_worker.deceased,
            deceased_datetime: db_worker.deceased_datetime,
            addresses,
            marital_status: db_worker.marital_status,
            multiple_birth: db_worker.multiple_birth,
            tax_id: None, // TODO: Load from DB
            documents: vec![], // TODO: Load from DB
            emergency_contacts: vec![], // TODO: Load from DB
            photo: vec![], // Not stored in DB yet
            managing_organization: db_worker.managing_organization_id,
            links,
            created_at: db_worker.created_at,
            updated_at: db_worker.updated_at,
        })
    }

    /// Load all associated data for a worker
    async fn load_associations(&self, worker_id: &Uuid) -> Result<(
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

        Ok((db_names, db_identifiers, db_addresses, db_contacts, db_links))
    }
}

#[async_trait::async_trait]
impl WorkerRepository for SeaOrmWorkerRepository {
    async fn create(&self, worker: &Worker) -> Result<Worker> {
        let txn = self.db.begin().await?;

        let (new_worker, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            self.to_active_models(worker);

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

        txn.commit().await?;

        // Load associations
        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(&db_worker.id).await?;

        let result = self.from_db_models(db_worker, db_names, db_identifiers, db_addresses, db_contacts, db_links)?;

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Created {
            worker: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Ok(worker_json) = serde_json::to_value(&result) {
            self.log_audit("CREATE", result.id, None, Some(worker_json), &AuditContext::default()).await;
        }

        Ok(result)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Worker>> {
        let db_worker = workers::Entity::find_by_id(*id)
            .filter(workers::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;

        let db_worker = match db_worker {
            Some(p) => p,
            None => return Ok(None),
        };

        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(id).await?;

        self.from_db_models(db_worker, db_names, db_identifiers, db_addresses, db_contacts, db_links)
            .map(Some)
    }

    async fn update(&self, worker: &Worker) -> Result<Worker> {
        // Get old values for audit
        let old_worker = self.get_by_id(&worker.id).await?;

        let txn = self.db.begin().await?;

        // Update worker
        let update_model = workers::ActiveModel {
            id: Set(worker.id),
            active: Set(worker.active),
            worker_type: Set(worker.worker_type.as_ref().map(|wt| wt.to_string())),
            gender: Set(format!("{:?}", worker.gender)),
            birth_date: Set(worker.birth_date),
            deceased: Set(worker.deceased),
            deceased_datetime: Set(worker.deceased_datetime),
            marital_status: Set(worker.marital_status.clone()),
            multiple_birth: Set(worker.multiple_birth),
            managing_organization_id: Set(worker.managing_organization),
            updated_at: Set(Utc::now()),
            updated_by: Set(None),
            ..Default::default()
        };
        update_model.update(&txn).await?;

        // Delete existing associated data
        worker_names::Entity::delete_many()
            .filter(worker_names::Column::WorkerId.eq(worker.id))
            .exec(&txn).await?;

        worker_identifiers::Entity::delete_many()
            .filter(worker_identifiers::Column::WorkerId.eq(worker.id))
            .exec(&txn).await?;

        worker_addresses::Entity::delete_many()
            .filter(worker_addresses::Column::WorkerId.eq(worker.id))
            .exec(&txn).await?;

        worker_contacts::Entity::delete_many()
            .filter(worker_contacts::Column::WorkerId.eq(worker.id))
            .exec(&txn).await?;

        worker_links::Entity::delete_many()
            .filter(worker_links::Column::WorkerId.eq(worker.id))
            .exec(&txn).await?;

        // Re-insert associated data
        let (_, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            self.to_active_models(worker);

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

        // Fetch and return updated worker
        let result = self.get_by_id(&worker.id).await?
            .ok_or_else(|| crate::Error::Validation("Worker not found after update".to_string()))?;

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Updated {
            worker: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_json) = old_worker.as_ref().and_then(|p| serde_json::to_value(p).ok()) {
            if let Ok(new_json) = serde_json::to_value(&result) {
                self.log_audit("UPDATE", result.id, Some(old_json), Some(new_json), &AuditContext::default()).await;
            }
        }

        Ok(result)
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        // Get old values for audit
        let old_worker = self.get_by_id(id).await?;

        // Soft delete
        let update_model = workers::ActiveModel {
            id: Set(*id),
            deleted_at: Set(Some(Utc::now())),
            deleted_by: Set(Some("system".to_string())),
            ..Default::default()
        };
        update_model.update(&self.db).await?;

        // Publish event
        self.publish_event(crate::streaming::WorkerEvent::Deleted {
            worker_id: *id,
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_worker) = old_worker {
            if let Ok(old_json) = serde_json::to_value(&old_worker) {
                self.log_audit("DELETE", *id, Some(old_json), None, &AuditContext::default()).await;
            }
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Worker>> {
        let search_pattern = format!("%{}%", query.to_lowercase());

        let worker_ids: Vec<Uuid> = worker_names::Entity::find()
            .filter(Expr::cust_with_values("LOWER(family) LIKE $1", [search_pattern]))
            .select_only()
            .column(worker_names::Column::WorkerId)
            .distinct()
            .into_tuple()
            .all(&self.db)
            .await?;

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

        let mut workers = Vec::new();
        for db_worker in db_workers {
            if let Some(worker) = self.get_by_id(&db_worker.id).await? {
                workers.push(worker);
            }
        }

        Ok(workers)
    }
}
