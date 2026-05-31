//! NHS ODS (Organisation Data Service) domain types
//!
//! Models for organization roles, relationships, succession,
//! and ODS-specific reference data aligned with the NHS England
//! Organisation Data Terminology FHIR API.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// ODS record status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OdsStatus {
    Active,
    Inactive,
}

/// ODS record class — whether the record is an organisation or a site
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordClass {
    /// RC1 — full organisation record
    Organisation,
    /// RC2 — site record
    Site,
}

/// ODS record use type — whether the record is a full record or reference-only
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecordUseType {
    /// Full record with complete data
    Full,
    /// Minimal record included only for referential integrity
    RefOnly,
}

/// Date period type — whether a date range is legal or operational
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PeriodType {
    /// Legal existence period
    Legal,
    /// Operational (system-use) period
    Operational,
}

/// A typed date period with optional start and end dates
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DatePeriod {
    pub period_type: PeriodType,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

/// A role assigned to an organisation or site (e.g. RO197 = NHS Trust)
///
/// Every organisation/site must have exactly one primary role.
/// Additional non-primary roles may also be assigned.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrganizationRole {
    /// Unique integer ID for this specific role assignment
    pub unique_role_id: i64,
    /// Role code (e.g. "RO197", "RO76")
    pub role_code: String,
    /// Human-readable role name (e.g. "NHS Trust", "GP Practice")
    pub role_name: Option<String>,
    /// Whether this is the primary role (exactly one per org)
    pub is_primary: bool,
    /// Active or Inactive
    pub status: OdsStatus,
    /// Legal and/or operational date periods
    pub periods: Vec<DatePeriod>,
}

/// A directional relationship from one organisation to another
///
/// Held on the "source" entity pointing to a "target".
/// E.g. RE4 "IS COMMISSIONED BY", RE6 "IS OPERATED BY"
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrganizationRelationship {
    /// Unique integer ID for this relationship instance
    pub unique_rel_id: i64,
    /// Relationship type code (e.g. "RE4", "RE6")
    pub relationship_type_code: String,
    /// Human-readable name (e.g. "IS COMMISSIONED BY")
    pub relationship_type_name: Option<String>,
    /// Active or Inactive
    pub status: OdsStatus,
    /// Target organisation ODS code
    pub target_ods_code: String,
    /// Target's primary role at time of relationship
    pub target_primary_role_id: Option<String>,
    /// Legal and/or operational date periods
    pub periods: Vec<DatePeriod>,
}

/// Succession type — whether this record points to a predecessor or successor
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuccessionType {
    /// The target organisation preceded this one (was absorbed/closed)
    Predecessor,
    /// The target organisation succeeded this one (took over)
    Successor,
}

/// A succession record linking organisations through mergers and acquisitions
///
/// References are held on both sides. Only immediate links are stored;
/// longer chains require sequential querying.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OrganizationSuccession {
    /// Unique integer ID for this succession record
    pub unique_succ_id: i64,
    /// Whether the target is a predecessor or successor
    pub succession_type: SuccessionType,
    /// Target organisation ODS code
    pub target_ods_code: String,
    /// Target's primary role at time of succession
    pub target_primary_role_id: Option<String>,
    /// Legal start date of the succession
    pub legal_start_date: Option<NaiveDate>,
    /// Flag indicating whether the target has also been succeeded
    pub has_forward_succession: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ods_status_serialization() {
        let statuses = vec![OdsStatus::Active, OdsStatus::Inactive];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let deser: OdsStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, s);
        }
    }

    #[test]
    fn test_record_class_serialization() {
        let classes = vec![RecordClass::Organisation, RecordClass::Site];
        for c in classes {
            let json = serde_json::to_string(&c).unwrap();
            let deser: RecordClass = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, c);
        }
    }

    #[test]
    fn test_period_type_serialization() {
        let types = vec![PeriodType::Legal, PeriodType::Operational];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let deser: PeriodType = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, t);
        }
    }

    #[test]
    fn test_date_period_construction() {
        let period = DatePeriod {
            period_type: PeriodType::Legal,
            start_date: Some(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            end_date: None,
        };
        assert!(period.end_date.is_none());
        assert_eq!(period.period_type, PeriodType::Legal);
    }

    #[test]
    fn test_organization_role_construction() {
        let role = OrganizationRole {
            unique_role_id: 12345,
            role_code: "RO197".to_string(),
            role_name: Some("NHS Trust".to_string()),
            is_primary: true,
            status: OdsStatus::Active,
            periods: vec![DatePeriod {
                period_type: PeriodType::Legal,
                start_date: Some(chrono::NaiveDate::from_ymd_opt(2000, 4, 1).unwrap()),
                end_date: None,
            }],
        };
        assert!(role.is_primary);
        assert_eq!(role.role_code, "RO197");
    }

    #[test]
    fn test_organization_relationship_construction() {
        let rel = OrganizationRelationship {
            unique_rel_id: 67890,
            relationship_type_code: "RE4".to_string(),
            relationship_type_name: Some("IS COMMISSIONED BY".to_string()),
            status: OdsStatus::Active,
            target_ods_code: "X26".to_string(),
            target_primary_role_id: Some("RO209".to_string()),
            periods: vec![],
        };
        assert_eq!(rel.relationship_type_code, "RE4");
        assert_eq!(rel.target_ods_code, "X26");
    }

    #[test]
    fn test_organization_succession_construction() {
        let succ = OrganizationSuccession {
            unique_succ_id: 11111,
            succession_type: SuccessionType::Predecessor,
            target_ods_code: "RAV".to_string(),
            target_primary_role_id: Some("RO197".to_string()),
            legal_start_date: Some(chrono::NaiveDate::from_ymd_opt(1993, 4, 1).unwrap()),
            has_forward_succession: false,
        };
        assert_eq!(succ.succession_type, SuccessionType::Predecessor);
        assert!(!succ.has_forward_succession);
    }

    #[test]
    fn test_succession_type_serialization() {
        let types = vec![SuccessionType::Predecessor, SuccessionType::Successor];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let deser: SuccessionType = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, t);
        }
    }

    #[test]
    fn test_record_use_type_serialization() {
        let types = vec![RecordUseType::Full, RecordUseType::RefOnly];
        for t in types {
            let json = serde_json::to_string(&t).unwrap();
            let deser: RecordUseType = serde_json::from_str(&json).unwrap();
            assert_eq!(deser, t);
        }
    }
}
