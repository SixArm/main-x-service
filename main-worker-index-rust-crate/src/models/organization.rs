//! Organization model definition
//!
//! Supports NHS ODS (Organisation Data Service) fields including
//! ODS code, record class, assigning authority, roles, relationships,
//! succession records, and geographic boundary data.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use super::{Address, ContactPoint, Identifier};
use super::ods::{
    OdsStatus, RecordClass, RecordUseType, OrganizationRole,
    OrganizationRelationship, OrganizationSuccession, DatePeriod, SuccessionType,
};

/// Organization (hospital, trust, GP practice, ICB, site, etc.)
///
/// Aligned with the NHS ODS data model. An ODS code is a unique
/// identification code for an organisation that interacts with any
/// area of the NHS.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Organization {
    /// Unique internal identifier (UUID)
    pub id: Uuid,

    /// Organization identifiers (ODS code, NPI, etc.)
    pub identifiers: Vec<Identifier>,

    /// Active status
    pub active: bool,

    /// ODS code — unique identification code (max 12 chars, never reused)
    #[serde(default)]
    pub ods_code: Option<String>,

    /// ODS record status
    #[serde(default)]
    pub ods_status: Option<OdsStatus>,

    /// Record class: Organisation (RC1) or Site (RC2)
    #[serde(default)]
    pub record_class: Option<RecordClass>,

    /// Record use type: Full or RefOnly
    #[serde(default)]
    pub record_use_type: Option<RecordUseType>,

    /// Assigning authority — the authority managing this ODS code range
    #[serde(default)]
    pub assigning_authority: Option<String>,

    /// Organization type (Hospital, Clinic, etc.)
    pub org_type: Vec<String>,

    /// Organization name
    pub name: String,

    /// Alias names
    pub alias: Vec<String>,

    /// Telecom contacts
    pub telecom: Vec<ContactPoint>,

    /// Addresses
    pub addresses: Vec<Address>,

    /// Part of (parent organization UUID)
    pub part_of: Option<Uuid>,

    /// Legal and operational date periods
    #[serde(default)]
    pub periods: Vec<DatePeriod>,

    /// Last change date from ODS
    #[serde(default)]
    pub last_change_date: Option<NaiveDate>,

    /// Roles assigned to this organisation (primary + non-primary)
    #[serde(default)]
    pub roles: Vec<OrganizationRole>,

    /// Relationships to other organisations
    #[serde(default)]
    pub relationships: Vec<OrganizationRelationship>,

    /// Succession records (predecessors and successors)
    #[serde(default)]
    pub successions: Vec<OrganizationSuccession>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    /// Create a new organization
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            identifiers: Vec::new(),
            active: true,
            ods_code: None,
            ods_status: None,
            record_class: None,
            record_use_type: None,
            assigning_authority: None,
            org_type: Vec::new(),
            name,
            alias: Vec::new(),
            telecom: Vec::new(),
            addresses: Vec::new(),
            part_of: None,
            periods: Vec::new(),
            last_change_date: None,
            roles: Vec::new(),
            relationships: Vec::new(),
            successions: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the primary role for this organisation, if any
    pub fn primary_role(&self) -> Option<&OrganizationRole> {
        self.roles.iter().find(|r| r.is_primary)
    }

    /// Get active relationships only
    pub fn active_relationships(&self) -> Vec<&OrganizationRelationship> {
        self.relationships.iter().filter(|r| r.status == OdsStatus::Active).collect()
    }

    /// Get predecessor organisations from succession records
    pub fn predecessors(&self) -> Vec<&OrganizationSuccession> {
        self.successions.iter()
            .filter(|s| s.succession_type == SuccessionType::Predecessor)
            .collect()
    }

    /// Get successor organisations from succession records
    pub fn successors(&self) -> Vec<&OrganizationSuccession> {
        self.successions.iter()
            .filter(|s| s.succession_type == SuccessionType::Successor)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ods::*;

    #[test]
    fn test_organization_new_defaults() {
        let org = Organization::new("NHS Trust".to_string());
        assert!(org.active);
        assert_eq!(org.name, "NHS Trust");
        assert!(org.ods_code.is_none());
        assert!(org.record_class.is_none());
        assert!(org.assigning_authority.is_none());
        assert!(org.roles.is_empty());
        assert!(org.relationships.is_empty());
        assert!(org.successions.is_empty());
        assert!(org.periods.is_empty());
    }

    #[test]
    fn test_organization_with_ods_fields() {
        let mut org = Organization::new("Guy's and St Thomas' NHS Foundation Trust".to_string());
        org.ods_code = Some("RJ1".to_string());
        org.ods_status = Some(OdsStatus::Active);
        org.record_class = Some(RecordClass::Organisation);
        org.record_use_type = Some(RecordUseType::Full);
        org.assigning_authority = Some("HSCIC".to_string());

        assert_eq!(org.ods_code.as_deref(), Some("RJ1"));
        assert_eq!(org.record_class, Some(RecordClass::Organisation));
        assert_eq!(org.assigning_authority.as_deref(), Some("HSCIC"));
    }

    #[test]
    fn test_organization_primary_role() {
        let mut org = Organization::new("Test Trust".to_string());
        org.roles.push(OrganizationRole {
            unique_role_id: 1,
            role_code: "RO197".to_string(),
            role_name: Some("NHS Trust".to_string()),
            is_primary: true,
            status: OdsStatus::Active,
            periods: vec![],
        });
        org.roles.push(OrganizationRole {
            unique_role_id: 2,
            role_code: "RO24".to_string(),
            role_name: Some("Acute Trust".to_string()),
            is_primary: false,
            status: OdsStatus::Active,
            periods: vec![],
        });

        let primary = org.primary_role().unwrap();
        assert_eq!(primary.role_code, "RO197");
    }

    #[test]
    fn test_organization_predecessors_successors() {
        let mut org = Organization::new("Merged Trust".to_string());
        org.successions.push(OrganizationSuccession {
            unique_succ_id: 1,
            succession_type: SuccessionType::Predecessor,
            target_ods_code: "OLD1".to_string(),
            target_primary_role_id: Some("RO197".to_string()),
            legal_start_date: None,
            has_forward_succession: false,
        });
        org.successions.push(OrganizationSuccession {
            unique_succ_id: 2,
            succession_type: SuccessionType::Successor,
            target_ods_code: "NEW1".to_string(),
            target_primary_role_id: Some("RO197".to_string()),
            legal_start_date: None,
            has_forward_succession: true,
        });

        assert_eq!(org.predecessors().len(), 1);
        assert_eq!(org.successors().len(), 1);
        assert!(org.successors()[0].has_forward_succession);
    }

    #[test]
    fn test_organization_serialization_roundtrip() {
        let mut org = Organization::new("Test Org".to_string());
        org.ods_code = Some("ABC".to_string());
        org.record_class = Some(RecordClass::Site);

        let json = serde_json::to_string(&org).unwrap();
        let deser: Organization = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.ods_code.as_deref(), Some("ABC"));
        assert_eq!(deser.record_class, Some(RecordClass::Site));
    }

    #[test]
    fn test_organization_deserialize_without_ods_fields() {
        // Verify #[serde(default)] works for existing data without ODS fields
        let json = r#"{"id":"00000000-0000-0000-0000-000000000001","identifiers":[],"active":true,"org_type":[],"name":"Old Org","alias":[],"telecom":[],"addresses":[],"part_of":null,"created_at":"2024-01-01T00:00:00Z","updated_at":"2024-01-01T00:00:00Z"}"#;
        let org: Organization = serde_json::from_str(json).unwrap();
        assert_eq!(org.name, "Old Org");
        assert!(org.ods_code.is_none());
        assert!(org.roles.is_empty());
        assert!(org.successions.is_empty());
    }
}
