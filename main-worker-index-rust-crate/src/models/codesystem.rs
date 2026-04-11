//! ODS CodeSystem reference data types
//!
//! Reference data for NHS ODS CodeSystems including organisation roles,
//! relationships, record classes, practitioner roles, and geographic names.
//! These are aligned with the NHS England Organisation Data Terminology FHIR API.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// ODSOrganisationRole — reference data for Primary and Non-Primary Roles
///
/// CodeSystem: https://digital.nhs.uk/services/organisation-data-service/CodeSystem/ODSOrganisationRole
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OdsRoleReference {
    /// Role ID (e.g. "RO197", "RO76")
    pub role_id: String,
    /// Role Name (e.g. "NHS Trust", "GP Practice")
    pub role_name: String,
    /// Whether this is a primary role type or non-primary
    pub is_primary_role_type: bool,
}

/// ODSRelationship — reference data for relationship types
///
/// CodeSystem: https://digital.nhs.uk/services/organisation-data-service/CodeSystem/ODSRelationship
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OdsRelationshipReference {
    /// Relationship Type ID (e.g. "RE4", "RE6")
    pub relationship_id: String,
    /// Relationship Name (e.g. "IS COMMISSIONED BY", "IS OPERATED BY")
    pub relationship_name: String,
}

/// ODSRecordClass — reference data for record classes
///
/// CodeSystem: https://digital.nhs.uk/services/organisation-data-service/CodeSystem/ODSRecordClass
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OdsRecordClassReference {
    /// Record class code (e.g. "RC1", "RC2")
    pub code: String,
    /// Record class name (e.g. "Organisation", "Site")
    pub name: String,
}

/// ODSRecordUseType — reference data for record use types
///
/// Full = current complete records, RefOnly = minimal records for referential integrity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OdsRecordUseTypeReference {
    /// Use type code (e.g. "Full", "RefOnly")
    pub code: String,
    /// Use type name
    pub name: String,
}

/// practitioner-role — reference data for practitioner role types
///
/// Currently limited to prescriber types/roles and consultants.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PractitionerRoleReference {
    /// Role code (e.g. "PGP" for General Practitioner)
    pub role_code: String,
    /// Role name (e.g. "General Practitioner")
    pub role_name: String,
    /// Role category (e.g. "Prescriber", "Consultant")
    pub role_category: Option<String>,
}

/// geography-name — reference data for ONS identifiers
///
/// Names for ONS (Office for National Statistics) identifiers used within
/// the postcode CodeSystem.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeographyNameReference {
    /// ONS code (e.g. "E09000033")
    pub ons_code: String,
    /// Name (e.g. "Westminster")
    pub name: String,
    /// Geography type (e.g. "Local Authority", "ICB", "LSOA11")
    pub geography_type: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ods_role_reference() {
        let role = OdsRoleReference {
            role_id: "RO197".to_string(),
            role_name: "NHS Trust".to_string(),
            is_primary_role_type: true,
        };
        assert!(role.is_primary_role_type);
    }

    #[test]
    fn test_ods_relationship_reference() {
        let rel = OdsRelationshipReference {
            relationship_id: "RE4".to_string(),
            relationship_name: "IS COMMISSIONED BY".to_string(),
        };
        assert_eq!(rel.relationship_id, "RE4");
    }

    #[test]
    fn test_practitioner_role_reference() {
        let role = PractitionerRoleReference {
            role_code: "PGP".to_string(),
            role_name: "General Practitioner".to_string(),
            role_category: Some("Prescriber".to_string()),
        };
        assert_eq!(role.role_code, "PGP");
    }

    #[test]
    fn test_geography_name_reference() {
        let geo = GeographyNameReference {
            ons_code: "E09000033".to_string(),
            name: "Westminster".to_string(),
            geography_type: "Local Authority".to_string(),
        };
        assert_eq!(geo.geography_type, "Local Authority");
    }

    #[test]
    fn test_ods_record_class_reference() {
        let rc = OdsRecordClassReference {
            code: "RC1".to_string(),
            name: "Organisation".to_string(),
        };
        assert_eq!(rc.code, "RC1");
    }

    #[test]
    fn test_ods_role_reference_serialization() {
        let role = OdsRoleReference {
            role_id: "RO76".to_string(),
            role_name: "GP Practice".to_string(),
            is_primary_role_type: true,
        };
        let json = serde_json::to_string(&role).unwrap();
        let deser: OdsRoleReference = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.role_id, "RO76");
        assert!(deser.is_primary_role_type);
    }
}
