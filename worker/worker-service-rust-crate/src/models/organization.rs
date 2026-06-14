//! Organization model definition.
//!
//! [`Organization`] is the registry's record for an NHS-aligned organisation
//! (hospital, trust, GP practice, ICB, site, …). It carries the generic
//! identity fields (identifiers, name, addresses, telecom, parent link) plus a
//! block of NHS **ODS** (Organisation Data Service) metadata: the ODS code,
//! record class, assigning authority, roles, relationships, and succession
//! records. The ODS field shapes themselves live in the sibling
//! [`ods`](crate::models::ods) module; this file composes them onto the
//! aggregate and adds convenience accessors
//! ([`primary_role`](Organization::primary_role),
//! [`predecessors`](Organization::predecessors), …).
//!
//! Every ODS field is `#[serde(default)]` so JSON produced before the ODS
//! columns existed still deserializes (pinned by
//! `tests::test_organization_deserialize_without_ods_fields`).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::ods::{
    DatePeriod, OdsStatus, OrganizationRelationship, OrganizationRole, OrganizationSuccession,
    RecordClass, RecordUseType, SuccessionType,
};
use super::{Address, ContactPoint, Identifier};

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

    /// ODS code — unique identification code (max 12 chars, never reused).
    /// This is the canonical NHS organisation key (e.g. `"RJ1"`); ODS never
    /// re-uses a code, so a former code stays bound to its original org.
    // `#[serde(default)]` here (and on the ODS fields below) lets pre-ODS JSON
    // round-trip: the field simply defaults to `None`/empty when absent.
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

    /// Free-text organisation-type tags (e.g. `"Hospital"`, `"Clinic"`).
    pub org_type: Vec<String>,

    /// Primary display name of the organisation (always present).
    pub name: String,

    /// Alternative / trading / historical names for the organisation.
    pub alias: Vec<String>,

    /// Telecom contact points (phone, email, fax, …).
    pub telecom: Vec<ContactPoint>,

    /// Physical addresses (registered office, sites, …).
    pub addresses: Vec<Address>,

    /// Parent organisation, by internal [`Uuid`]; `None` for a top-level org.
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

    /// When this record was first created.
    pub created_at: DateTime<Utc>,

    /// When this record was last modified.
    pub updated_at: DateTime<Utc>,
}

impl Organization {
    /// Creates a new active organization with a fresh [`Uuid`] and current
    /// timestamps; all ODS fields and collections start empty/`None`.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::models::Organization;
    ///
    /// let org = Organization::new("Example NHS Trust".into());
    /// assert!(org.active);
    /// assert!(org.ods_code.is_none());
    /// ```
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

    /// Returns the organisation's primary role, if one is assigned.
    ///
    /// Per the ODS model each organisation has exactly one primary role; this
    /// returns the first role flagged [`is_primary`](OrganizationRole::is_primary).
    pub fn primary_role(&self) -> Option<&OrganizationRole> {
        self.roles.iter().find(|r| r.is_primary)
    }

    /// Returns only the relationships whose status is
    /// [`OdsStatus::Active`], filtering out inactive/historical ones.
    pub fn active_relationships(&self) -> Vec<&OrganizationRelationship> {
        self.relationships
            .iter()
            .filter(|r| r.status == OdsStatus::Active)
            .collect()
    }

    /// Returns succession records pointing to *predecessor* organisations
    /// (those this organisation absorbed or replaced).
    pub fn predecessors(&self) -> Vec<&OrganizationSuccession> {
        self.successions
            .iter()
            .filter(|s| s.succession_type == SuccessionType::Predecessor)
            .collect()
    }

    /// Returns succession records pointing to *successor* organisations
    /// (those that took over from this one).
    pub fn successors(&self) -> Vec<&OrganizationSuccession> {
        self.successions
            .iter()
            .filter(|s| s.succession_type == SuccessionType::Successor)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ods::*;

    /// `new` produces an active org with empty ODS fields and collections.
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

    /// ODS metadata fields can be set and read back.
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

    /// `primary_role` returns the single role flagged primary.
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

    /// `predecessors`/`successors` partition succession records by direction.
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

    /// An organization survives a JSON round-trip with ODS fields intact.
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

    /// Legacy JSON lacking ODS fields deserializes via `#[serde(default)]`.
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
