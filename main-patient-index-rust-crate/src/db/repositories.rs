//! Repository pattern implementations for database operations

use sea_orm::*;
use sea_orm::sea_query::Expr;
use chrono::Utc;
use uuid::Uuid;

use crate::models::{Patient, HumanName, Address, ContactPoint, Identifier, PatientLink};
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

/// Patient repository trait
#[async_trait::async_trait]
pub trait PatientRepository: Send + Sync {
    /// Create a new patient
    async fn create(&self, patient: &Patient) -> Result<Patient>;

    /// Get a patient by ID
    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Patient>>;

    /// Update a patient
    async fn update(&self, patient: &Patient) -> Result<Patient>;

    /// Delete a patient (soft delete)
    async fn delete(&self, id: &Uuid) -> Result<()>;

    /// Search patients by name
    async fn search(&self, query: &str) -> Result<Vec<Patient>>;

    /// List all active patients (non-deleted)
    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Patient>>;
}

/// SeaORM-based patient repository implementation
pub struct SeaOrmPatientRepository {
    db: DatabaseConnection,
    event_publisher: Option<std::sync::Arc<dyn crate::streaming::EventProducer>>,
    audit_log: Option<std::sync::Arc<super::audit::AuditLogRepository>>,
}

impl SeaOrmPatientRepository {
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
    fn publish_event(&self, event: crate::streaming::PatientEvent) {
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
                    "Patient",
                    entity_id,
                    new_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                "UPDATE" => audit_log.log_update(
                    "Patient",
                    entity_id,
                    old_values.unwrap_or(serde_json::Value::Null),
                    new_values.unwrap_or(serde_json::Value::Null),
                    context.user_id.clone(),
                    context.ip_address.clone(),
                    context.user_agent.clone(),
                ).await,
                "DELETE" => audit_log.log_delete(
                    "Patient",
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

    /// Convert domain Patient model to SeaORM active models
    fn to_active_models(&self, patient: &Patient) -> (
        patients::ActiveModel,
        Vec<patient_names::ActiveModel>,
        Vec<patient_identifiers::ActiveModel>,
        Vec<patient_addresses::ActiveModel>,
        Vec<patient_contacts::ActiveModel>,
        Vec<patient_links::ActiveModel>,
    ) {
        let new_patient = patients::ActiveModel {
            id: Set(patient.id),
            active: Set(patient.active),
            gender: Set(format!("{:?}", patient.gender)),
            birth_date: Set(patient.birth_date),
            deceased: Set(patient.deceased),
            deceased_datetime: Set(patient.deceased_datetime),
            marital_status: Set(patient.marital_status.clone()),
            multiple_birth: Set(patient.multiple_birth),
            managing_organization_id: Set(patient.managing_organization),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            created_by: Set(None),
            updated_by: Set(None),
            deleted_at: Set(None),
            deleted_by: Set(None),
        };

        // Primary name
        let mut names = vec![patient_names::ActiveModel {
            id: Set(Uuid::new_v4()),
            patient_id: Set(patient.id),
            use_type: Set(patient.name.use_type.as_ref().map(|u| format!("{:?}", u))),
            family: Set(patient.name.family.clone()),
            given: Set(patient.name.given.clone()),
            prefix: Set(patient.name.prefix.clone()),
            suffix: Set(patient.name.suffix.clone()),
            is_primary: Set(true),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }];

        // Additional names
        for add_name in &patient.additional_names {
            names.push(patient_names::ActiveModel {
                id: Set(Uuid::new_v4()),
                patient_id: Set(patient.id),
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
        let identifiers = patient.identifiers.iter().map(|id| patient_identifiers::ActiveModel {
            id: Set(Uuid::new_v4()),
            patient_id: Set(patient.id),
            use_type: Set(id.use_type.as_ref().map(|u| format!("{:?}", u))),
            identifier_type: Set(format!("{:?}", id.identifier_type)),
            system: Set(id.system.clone()),
            value: Set(id.value.clone()),
            assigner: Set(id.assigner.clone()),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Addresses
        let addresses = patient.addresses.iter().enumerate().map(|(idx, addr)| patient_addresses::ActiveModel {
            id: Set(Uuid::new_v4()),
            patient_id: Set(patient.id),
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
        let contacts = patient.telecom.iter().enumerate().map(|(idx, cp)| patient_contacts::ActiveModel {
            id: Set(Uuid::new_v4()),
            patient_id: Set(patient.id),
            system: Set(format!("{:?}", cp.system)),
            value: Set(cp.value.clone()),
            use_type: Set(cp.use_type.as_ref().map(|u| format!("{:?}", u))),
            is_primary: Set(idx == 0),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        }).collect();

        // Links
        let links = patient.links.iter().map(|link| patient_links::ActiveModel {
            id: Set(Uuid::new_v4()),
            patient_id: Set(patient.id),
            other_patient_id: Set(link.other_patient_id),
            link_type: Set(format!("{:?}", link.link_type)),
            created_at: Set(Utc::now()),
            created_by: Set(None),
        }).collect();

        (new_patient, names, identifiers, addresses, contacts, links)
    }

    /// Convert database models to domain Patient model
    fn from_db_models(
        &self,
        db_patient: patients::Model,
        db_names: Vec<patient_names::Model>,
        db_identifiers: Vec<patient_identifiers::Model>,
        db_addresses: Vec<patient_addresses::Model>,
        db_contacts: Vec<patient_contacts::Model>,
        db_links: Vec<patient_links::Model>,
    ) -> Result<Patient> {
        use crate::models::{Gender, NameUse, ContactPointSystem, ContactPointUse, LinkType, IdentifierType, IdentifierUse};

        // Parse gender
        let gender = match db_patient.gender.as_str() {
            "Male" => Gender::Male,
            "Female" => Gender::Female,
            "Other" => Gender::Other,
            _ => Gender::Unknown,
        };

        // Get primary name
        let primary_name = db_names.iter()
            .find(|n| n.is_primary)
            .ok_or_else(|| crate::Error::Validation("Patient has no primary name".to_string()))?;

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

                Some(PatientLink {
                    other_patient_id: link.other_patient_id,
                    link_type,
                })
            })
            .collect();

        Ok(Patient {
            id: db_patient.id,
            identifiers,
            active: db_patient.active,
            name,
            additional_names,
            telecom,
            gender,
            birth_date: db_patient.birth_date,
            deceased: db_patient.deceased,
            deceased_datetime: db_patient.deceased_datetime,
            addresses,
            marital_status: db_patient.marital_status,
            multiple_birth: db_patient.multiple_birth,
            tax_id: None, // TODO: Load from DB
            documents: vec![], // TODO: Load from DB
            emergency_contacts: vec![], // TODO: Load from DB
            photo: vec![], // Not stored in DB yet
            managing_organization: db_patient.managing_organization_id,
            links,
            created_at: db_patient.created_at,
            updated_at: db_patient.updated_at,
        })
    }

    /// Load all associated data for a patient
    async fn load_associations(&self, patient_id: &Uuid) -> Result<(
        Vec<patient_names::Model>,
        Vec<patient_identifiers::Model>,
        Vec<patient_addresses::Model>,
        Vec<patient_contacts::Model>,
        Vec<patient_links::Model>,
    )> {
        let db_names = patient_names::Entity::find()
            .filter(patient_names::Column::PatientId.eq(*patient_id))
            .all(&self.db)
            .await?;

        let db_identifiers = patient_identifiers::Entity::find()
            .filter(patient_identifiers::Column::PatientId.eq(*patient_id))
            .all(&self.db)
            .await?;

        let db_addresses = patient_addresses::Entity::find()
            .filter(patient_addresses::Column::PatientId.eq(*patient_id))
            .all(&self.db)
            .await?;

        let db_contacts = patient_contacts::Entity::find()
            .filter(patient_contacts::Column::PatientId.eq(*patient_id))
            .all(&self.db)
            .await?;

        let db_links = patient_links::Entity::find()
            .filter(patient_links::Column::PatientId.eq(*patient_id))
            .all(&self.db)
            .await?;

        Ok((db_names, db_identifiers, db_addresses, db_contacts, db_links))
    }
}

#[async_trait::async_trait]
impl PatientRepository for SeaOrmPatientRepository {
    async fn create(&self, patient: &Patient) -> Result<Patient> {
        let txn = self.db.begin().await?;

        let (new_patient, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            self.to_active_models(patient);

        // Insert patient
        let db_patient = new_patient.insert(&txn).await?;

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
            self.load_associations(&db_patient.id).await?;

        let result = self.from_db_models(db_patient, db_names, db_identifiers, db_addresses, db_contacts, db_links)?;

        // Publish event
        self.publish_event(crate::streaming::PatientEvent::Created {
            patient: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Ok(patient_json) = serde_json::to_value(&result) {
            self.log_audit("CREATE", result.id, None, Some(patient_json), &AuditContext::default()).await;
        }

        Ok(result)
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Patient>> {
        let db_patient = patients::Entity::find_by_id(*id)
            .filter(patients::Column::DeletedAt.is_null())
            .one(&self.db)
            .await?;

        let db_patient = match db_patient {
            Some(p) => p,
            None => return Ok(None),
        };

        let (db_names, db_identifiers, db_addresses, db_contacts, db_links) =
            self.load_associations(id).await?;

        self.from_db_models(db_patient, db_names, db_identifiers, db_addresses, db_contacts, db_links)
            .map(Some)
    }

    async fn update(&self, patient: &Patient) -> Result<Patient> {
        // Get old values for audit
        let old_patient = self.get_by_id(&patient.id).await?;

        let txn = self.db.begin().await?;

        // Update patient
        let update_model = patients::ActiveModel {
            id: Set(patient.id),
            active: Set(patient.active),
            gender: Set(format!("{:?}", patient.gender)),
            birth_date: Set(patient.birth_date),
            deceased: Set(patient.deceased),
            deceased_datetime: Set(patient.deceased_datetime),
            marital_status: Set(patient.marital_status.clone()),
            multiple_birth: Set(patient.multiple_birth),
            managing_organization_id: Set(patient.managing_organization),
            updated_at: Set(Utc::now()),
            updated_by: Set(None),
            ..Default::default()
        };
        update_model.update(&txn).await?;

        // Delete existing associated data
        patient_names::Entity::delete_many()
            .filter(patient_names::Column::PatientId.eq(patient.id))
            .exec(&txn).await?;

        patient_identifiers::Entity::delete_many()
            .filter(patient_identifiers::Column::PatientId.eq(patient.id))
            .exec(&txn).await?;

        patient_addresses::Entity::delete_many()
            .filter(patient_addresses::Column::PatientId.eq(patient.id))
            .exec(&txn).await?;

        patient_contacts::Entity::delete_many()
            .filter(patient_contacts::Column::PatientId.eq(patient.id))
            .exec(&txn).await?;

        patient_links::Entity::delete_many()
            .filter(patient_links::Column::PatientId.eq(patient.id))
            .exec(&txn).await?;

        // Re-insert associated data
        let (_, new_names, new_identifiers, new_addresses, new_contacts, new_links) =
            self.to_active_models(patient);

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

        // Fetch and return updated patient
        let result = self.get_by_id(&patient.id).await?
            .ok_or_else(|| crate::Error::Validation("Patient not found after update".to_string()))?;

        // Publish event
        self.publish_event(crate::streaming::PatientEvent::Updated {
            patient: result.clone(),
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_json) = old_patient.as_ref().and_then(|p| serde_json::to_value(p).ok()) {
            if let Ok(new_json) = serde_json::to_value(&result) {
                self.log_audit("UPDATE", result.id, Some(old_json), Some(new_json), &AuditContext::default()).await;
            }
        }

        Ok(result)
    }

    async fn delete(&self, id: &Uuid) -> Result<()> {
        // Get old values for audit
        let old_patient = self.get_by_id(id).await?;

        // Soft delete
        let update_model = patients::ActiveModel {
            id: Set(*id),
            deleted_at: Set(Some(Utc::now())),
            deleted_by: Set(Some("system".to_string())),
            ..Default::default()
        };
        update_model.update(&self.db).await?;

        // Publish event
        self.publish_event(crate::streaming::PatientEvent::Deleted {
            patient_id: *id,
            timestamp: chrono::Utc::now(),
        });

        // Log audit
        if let Some(old_patient) = old_patient {
            if let Ok(old_json) = serde_json::to_value(&old_patient) {
                self.log_audit("DELETE", *id, Some(old_json), None, &AuditContext::default()).await;
            }
        }

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<Patient>> {
        let search_pattern = format!("%{}%", query.to_lowercase());

        let patient_ids: Vec<Uuid> = patient_names::Entity::find()
            .filter(Expr::cust_with_values("LOWER(family) LIKE $1", [search_pattern]))
            .select_only()
            .column(patient_names::Column::PatientId)
            .distinct()
            .into_tuple()
            .all(&self.db)
            .await?;

        let mut patients = Vec::new();
        for patient_id in patient_ids {
            if let Some(patient) = self.get_by_id(&patient_id).await? {
                patients.push(patient);
            }
        }

        Ok(patients)
    }

    async fn list_active(&self, limit: u64, offset: u64) -> Result<Vec<Patient>> {
        let db_patients: Vec<patients::Model> = patients::Entity::find()
            .filter(patients::Column::DeletedAt.is_null())
            .filter(patients::Column::Active.eq(true))
            .limit(limit)
            .offset(offset)
            .all(&self.db)
            .await?;

        let mut patients = Vec::new();
        for db_patient in db_patients {
            if let Some(patient) = self.get_by_id(&db_patient.id).await? {
                patients.push(patient);
            }
        }

        Ok(patients)
    }
}
