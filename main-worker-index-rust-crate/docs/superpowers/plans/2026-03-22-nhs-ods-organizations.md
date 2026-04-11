# NHS ODS Organizations & CodeSystems Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand the Organization model and add NHS ODS-aligned entities for roles, relationships, succession, geographic boundaries, and reference CodeSystems.

**Architecture:** The Organization model gains ODS-style fields (ods_code, status, record_class, assigning_authority, date periods). New domain models are added for OrganizationRole, OrganizationRelationship, OrganizationSuccession, PostcodeGeography, and ODS CodeSystem reference data. Each entity gets a domain model, DB migration, SeaORM entity, and unit tests. The design follows the existing pattern: domain models in `src/models/`, DB entities in `src/db/models.rs`, migrations in `migrations/`.

**Tech Stack:** Rust, SeaORM, PostgreSQL, serde, chrono, uuid, utoipa (OpenAPI)

---

## File Structure

### New Files
- `src/models/ods.rs` — ODS-specific types: OrganizationRole, OrganizationRelationship, OrganizationSuccession, DatePeriod, PeriodType, RecordClass, RecordUseType, OdsStatus
- `src/models/geography.rs` — PostcodeGeography model with boundary mappings
- `src/models/codesystem.rs` — CodeSystem reference data types (ODSOrganisationRole, ODSRelationship, ODSRecordClass, ODSRecordUseType, PractitionerRole, GeographyName)
- `migrations/2024122800000007_expand_organizations_ods/up.sql` — Add ODS columns to organizations table
- `migrations/2024122800000007_expand_organizations_ods/down.sql` — Reverse
- `migrations/2024122800000008_create_ods_tables/up.sql` — Create organization_roles, organization_relationships, organization_successions, postcode_geography tables
- `migrations/2024122800000008_create_ods_tables/down.sql` — Reverse
- `migrations/2024122800000009_create_codesystem_tables/up.sql` — Create ODS CodeSystem reference tables
- `migrations/2024122800000009_create_codesystem_tables/down.sql` — Reverse

### Modified Files
- `src/models/organization.rs` — Add ODS fields, roles, relationships, successions, geographic data
- `src/models/mod.rs` — Register new modules, add re-exports
- `src/models/identifier.rs` — Add ODS IdentifierType variant
- `src/db/models.rs` — Add SeaORM entities for new tables, update organizations entity

---

### Task 1: ODS Domain Types (`src/models/ods.rs`)

**Files:**
- Create: `src/models/ods.rs`
- Modify: `src/models/mod.rs`

- [ ] **Step 1: Write tests for ODS types**

Add to the bottom of `src/models/ods.rs`:

```rust
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
```

- [ ] **Step 2: Write ODS type definitions**

Create `src/models/ods.rs`:

```rust
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
```

- [ ] **Step 3: Register module in mod.rs**

Add to `src/models/mod.rs`:

```rust
pub mod ods;
pub use ods::{
    OdsStatus, RecordClass, RecordUseType, PeriodType, DatePeriod,
    OrganizationRole, OrganizationRelationship, OrganizationSuccession, SuccessionType,
};
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib ods::`
Expected: PASS — all 9 ODS type tests pass

- [ ] **Step 5: Commit**

```bash
git add src/models/ods.rs src/models/mod.rs
git commit -m "feat: add NHS ODS domain types (roles, relationships, succession)"
```

---

### Task 2: Geography Model (`src/models/geography.rs`)

**Files:**
- Create: `src/models/geography.rs`
- Modify: `src/models/mod.rs`

- [ ] **Step 1: Write tests for PostcodeGeography**

Add to `src/models/geography.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postcode_geography_construction() {
        let geo = PostcodeGeography {
            postcode: "SW1A 1AA".to_string(),
            lsoa11: Some("E01004736".to_string()),
            local_authority: Some("E09000033".to_string()),
            local_authority_name: Some("Westminster".to_string()),
            icb: Some("QWE".to_string()),
            icb_name: Some("NHS North West London ICB".to_string()),
            nhs_england_region: Some("Y56".to_string()),
            nhs_england_region_name: Some("London".to_string()),
            parliamentary_constituency: Some("E14000639".to_string()),
            parliamentary_constituency_name: Some("Cities of London and Westminster".to_string()),
            government_office_region: Some("E12000007".to_string()),
            cancer_alliance: None,
        };
        assert_eq!(geo.postcode, "SW1A 1AA");
        assert_eq!(geo.local_authority_name.as_deref(), Some("Westminster"));
    }

    #[test]
    fn test_postcode_geography_serialization() {
        let geo = PostcodeGeography {
            postcode: "LS1 4AP".to_string(),
            lsoa11: Some("E01011229".to_string()),
            local_authority: Some("E08000035".to_string()),
            local_authority_name: Some("Leeds".to_string()),
            icb: Some("X2C4Y".to_string()),
            icb_name: Some("NHS West Yorkshire ICB".to_string()),
            nhs_england_region: None,
            nhs_england_region_name: None,
            parliamentary_constituency: None,
            parliamentary_constituency_name: None,
            government_office_region: None,
            cancer_alliance: None,
        };
        let json = serde_json::to_string(&geo).unwrap();
        let deser: PostcodeGeography = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.postcode, "LS1 4AP");
        assert_eq!(deser.icb_name.as_deref(), Some("NHS West Yorkshire ICB"));
    }
}
```

- [ ] **Step 2: Write PostcodeGeography model**

Create `src/models/geography.rs`:

```rust
//! Geographic boundary mappings for postcodes
//!
//! Maps postcodes to health and social care boundaries including
//! Local Authority, Integrated Care Board (ICB), LSOA, and others.
//! Aligned with NHS ODS postcode CodeSystem.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Geographic boundary mappings for a postcode
///
/// Contains various geographic values/boundaries for a postcode including
/// which Lower Super Output Area, Local Authority, Parliamentary Constituency,
/// and Integrated Care Board boundary a postcode falls within.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PostcodeGeography {
    /// The postcode (e.g. "SW1A 1AA")
    pub postcode: String,

    /// 2011 Lower Super Output Area code (e.g. "E01004736")
    pub lsoa11: Option<String>,

    /// Local Authority code (e.g. "E09000033")
    pub local_authority: Option<String>,

    /// Local Authority name (e.g. "Westminster")
    pub local_authority_name: Option<String>,

    /// Integrated Care Board code
    pub icb: Option<String>,

    /// Integrated Care Board name
    pub icb_name: Option<String>,

    /// NHS England Region code
    pub nhs_england_region: Option<String>,

    /// NHS England Region name
    pub nhs_england_region_name: Option<String>,

    /// Parliamentary Constituency code
    pub parliamentary_constituency: Option<String>,

    /// Parliamentary Constituency name
    pub parliamentary_constituency_name: Option<String>,

    /// Government Office Region code
    pub government_office_region: Option<String>,

    /// Cancer Alliance code
    pub cancer_alliance: Option<String>,
}
```

- [ ] **Step 3: Register module in mod.rs**

Add to `src/models/mod.rs`:

```rust
pub mod geography;
pub use geography::PostcodeGeography;
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib geography::`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/models/geography.rs src/models/mod.rs
git commit -m "feat: add PostcodeGeography model for NHS boundary mappings"
```

---

### Task 3: CodeSystem Reference Data (`src/models/codesystem.rs`)

**Files:**
- Create: `src/models/codesystem.rs`
- Modify: `src/models/mod.rs`

- [ ] **Step 1: Write tests for CodeSystem types**

Add to `src/models/codesystem.rs`:

```rust
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
```

- [ ] **Step 2: Write CodeSystem reference types**

Create `src/models/codesystem.rs`:

```rust
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
```

- [ ] **Step 3: Register module in mod.rs**

Add to `src/models/mod.rs`:

```rust
pub mod codesystem;
pub use codesystem::{
    OdsRoleReference, OdsRelationshipReference, OdsRecordClassReference,
    OdsRecordUseTypeReference, PractitionerRoleReference, GeographyNameReference,
};
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib codesystem::`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/models/codesystem.rs src/models/mod.rs
git commit -m "feat: add ODS CodeSystem reference data types"
```

---

### Task 4: Expand Organization Model

**Files:**
- Modify: `src/models/organization.rs`
- Modify: `src/models/identifier.rs`

- [ ] **Step 1: Add ODS identifier type**

In `src/models/identifier.rs`, add an `ODS` variant to `IdentifierType`:

```rust
/// ODS Organisation Code
ODS,
```

And in the Display impl:
```rust
IdentifierType::ODS => write!(f, "ODS"),
```

- [ ] **Step 2: Expand Organization struct with ODS fields**

Update `src/models/organization.rs` to add ODS-specific fields:

```rust
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
    OrganizationRelationship, OrganizationSuccession, DatePeriod,
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
            .filter(|s| s.succession_type == super::ods::SuccessionType::Predecessor)
            .collect()
    }

    /// Get successor organisations from succession records
    pub fn successors(&self) -> Vec<&OrganizationSuccession> {
        self.successions.iter()
            .filter(|s| s.succession_type == super::ods::SuccessionType::Successor)
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
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --lib organization:: && cargo test --lib ods::`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/models/organization.rs src/models/identifier.rs
git commit -m "feat: expand Organization model with ODS fields, roles, relationships, succession"
```

---

### Task 5: Database Migration — Expand Organizations Table

**Files:**
- Create: `migrations/2024122800000007_expand_organizations_ods/up.sql`
- Create: `migrations/2024122800000007_expand_organizations_ods/down.sql`

- [ ] **Step 1: Write up migration**

```sql
-- Expand organizations table with NHS ODS fields

ALTER TABLE organizations ADD COLUMN ods_code VARCHAR(12) UNIQUE;
ALTER TABLE organizations ADD COLUMN ods_status VARCHAR(20)
    CHECK (ods_status IN ('active', 'inactive'));
ALTER TABLE organizations ADD COLUMN record_class VARCHAR(20)
    CHECK (record_class IN ('organisation', 'site'));
ALTER TABLE organizations ADD COLUMN record_use_type VARCHAR(20)
    CHECK (record_use_type IN ('full', 'ref_only'));
ALTER TABLE organizations ADD COLUMN assigning_authority VARCHAR(255);
ALTER TABLE organizations ADD COLUMN last_change_date DATE;

-- Indexes
CREATE INDEX idx_organizations_ods_code ON organizations(ods_code);
CREATE INDEX idx_organizations_record_class ON organizations(record_class);
CREATE INDEX idx_organizations_ods_status ON organizations(ods_status);
```

- [ ] **Step 2: Write down migration**

```sql
DROP INDEX IF EXISTS idx_organizations_ods_status;
DROP INDEX IF EXISTS idx_organizations_record_class;
DROP INDEX IF EXISTS idx_organizations_ods_code;

ALTER TABLE organizations DROP COLUMN IF EXISTS last_change_date;
ALTER TABLE organizations DROP COLUMN IF EXISTS assigning_authority;
ALTER TABLE organizations DROP COLUMN IF EXISTS record_use_type;
ALTER TABLE organizations DROP COLUMN IF EXISTS record_class;
ALTER TABLE organizations DROP COLUMN IF EXISTS ods_status;
ALTER TABLE organizations DROP COLUMN IF EXISTS ods_code;
```

- [ ] **Step 3: Commit**

```bash
git add migrations/2024122800000007_expand_organizations_ods/
git commit -m "feat: migration to add ODS columns to organizations table"
```

---

### Task 6: Database Migration — ODS Instance Tables

**Files:**
- Create: `migrations/2024122800000008_create_ods_tables/up.sql`
- Create: `migrations/2024122800000008_create_ods_tables/down.sql`

- [ ] **Step 1: Write up migration**

```sql
-- Organization roles (primary + non-primary)
CREATE TABLE organization_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    unique_role_id BIGINT NOT NULL,
    role_code VARCHAR(20) NOT NULL,
    role_name VARCHAR(255),
    is_primary BOOLEAN NOT NULL DEFAULT false,
    status VARCHAR(20) NOT NULL CHECK (status IN ('active', 'inactive')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, unique_role_id)
);

-- Role date periods
CREATE TABLE organization_role_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_role_id UUID NOT NULL REFERENCES organization_roles(id) ON DELETE CASCADE,
    period_type VARCHAR(20) NOT NULL CHECK (period_type IN ('legal', 'operational')),
    start_date DATE,
    end_date DATE
);

-- Organization relationships
CREATE TABLE organization_relationships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    unique_rel_id BIGINT NOT NULL,
    relationship_type_code VARCHAR(20) NOT NULL,
    relationship_type_name VARCHAR(255),
    status VARCHAR(20) NOT NULL CHECK (status IN ('active', 'inactive')),
    target_ods_code VARCHAR(12) NOT NULL,
    target_primary_role_id VARCHAR(20),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, unique_rel_id)
);

-- Relationship date periods
CREATE TABLE organization_relationship_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_relationship_id UUID NOT NULL REFERENCES organization_relationships(id) ON DELETE CASCADE,
    period_type VARCHAR(20) NOT NULL CHECK (period_type IN ('legal', 'operational')),
    start_date DATE,
    end_date DATE
);

-- Organization successions (mergers, acquisitions)
CREATE TABLE organization_successions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    unique_succ_id BIGINT NOT NULL,
    succession_type VARCHAR(20) NOT NULL CHECK (succession_type IN ('predecessor', 'successor')),
    target_ods_code VARCHAR(12) NOT NULL,
    target_primary_role_id VARCHAR(20),
    legal_start_date DATE,
    has_forward_succession BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(organization_id, unique_succ_id)
);

-- Organization date periods (legal/operational existence)
CREATE TABLE organization_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_type VARCHAR(20) NOT NULL CHECK (period_type IN ('legal', 'operational')),
    start_date DATE,
    end_date DATE
);

-- Postcode geography boundary mappings
CREATE TABLE postcode_geography (
    postcode VARCHAR(10) PRIMARY KEY,
    lsoa11 VARCHAR(20),
    local_authority VARCHAR(20),
    local_authority_name VARCHAR(255),
    icb VARCHAR(20),
    icb_name VARCHAR(255),
    nhs_england_region VARCHAR(20),
    nhs_england_region_name VARCHAR(255),
    parliamentary_constituency VARCHAR(20),
    parliamentary_constituency_name VARCHAR(255),
    government_office_region VARCHAR(20),
    cancer_alliance VARCHAR(20),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_org_roles_org_id ON organization_roles(organization_id);
CREATE INDEX idx_org_roles_code ON organization_roles(role_code);
CREATE INDEX idx_org_roles_primary ON organization_roles(is_primary);
CREATE INDEX idx_org_rels_org_id ON organization_relationships(organization_id);
CREATE INDEX idx_org_rels_target ON organization_relationships(target_ods_code);
CREATE INDEX idx_org_rels_type ON organization_relationships(relationship_type_code);
CREATE INDEX idx_org_succs_org_id ON organization_successions(organization_id);
CREATE INDEX idx_org_succs_target ON organization_successions(target_ods_code);
CREATE INDEX idx_org_succs_type ON organization_successions(succession_type);
CREATE INDEX idx_org_periods_org_id ON organization_periods(organization_id);
CREATE INDEX idx_postcode_geo_la ON postcode_geography(local_authority);
CREATE INDEX idx_postcode_geo_icb ON postcode_geography(icb);
CREATE INDEX idx_postcode_geo_lsoa ON postcode_geography(lsoa11);
```

- [ ] **Step 2: Write down migration**

```sql
DROP INDEX IF EXISTS idx_postcode_geo_lsoa;
DROP INDEX IF EXISTS idx_postcode_geo_icb;
DROP INDEX IF EXISTS idx_postcode_geo_la;
DROP INDEX IF EXISTS idx_org_periods_org_id;
DROP INDEX IF EXISTS idx_org_succs_type;
DROP INDEX IF EXISTS idx_org_succs_target;
DROP INDEX IF EXISTS idx_org_succs_org_id;
DROP INDEX IF EXISTS idx_org_rels_type;
DROP INDEX IF EXISTS idx_org_rels_target;
DROP INDEX IF EXISTS idx_org_rels_org_id;
DROP INDEX IF EXISTS idx_org_roles_primary;
DROP INDEX IF EXISTS idx_org_roles_code;
DROP INDEX IF EXISTS idx_org_roles_org_id;

DROP TABLE IF EXISTS postcode_geography;
DROP TABLE IF EXISTS organization_periods;
DROP TABLE IF EXISTS organization_successions;
DROP TABLE IF EXISTS organization_relationship_periods;
DROP TABLE IF EXISTS organization_relationships;
DROP TABLE IF EXISTS organization_role_periods;
DROP TABLE IF EXISTS organization_roles;
```

- [ ] **Step 3: Commit**

```bash
git add migrations/2024122800000008_create_ods_tables/
git commit -m "feat: migration for ODS roles, relationships, succession, geography tables"
```

---

### Task 7: Database Migration — CodeSystem Reference Tables

**Files:**
- Create: `migrations/2024122800000009_create_codesystem_tables/up.sql`
- Create: `migrations/2024122800000009_create_codesystem_tables/down.sql`

- [ ] **Step 1: Write up migration**

```sql
-- ODS CodeSystem reference tables

-- ODSOrganisationRole reference data
CREATE TABLE ods_role_references (
    role_id VARCHAR(20) PRIMARY KEY,
    role_name VARCHAR(255) NOT NULL,
    is_primary_role_type BOOLEAN NOT NULL DEFAULT false,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ODSRelationship reference data
CREATE TABLE ods_relationship_references (
    relationship_id VARCHAR(20) PRIMARY KEY,
    relationship_name VARCHAR(255) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ODSRecordClass reference data
CREATE TABLE ods_record_class_references (
    code VARCHAR(10) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ODSRecordUseType reference data
CREATE TABLE ods_record_use_type_references (
    code VARCHAR(20) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Practitioner role reference data
CREATE TABLE practitioner_role_references (
    role_code VARCHAR(20) PRIMARY KEY,
    role_name VARCHAR(255) NOT NULL,
    role_category VARCHAR(100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Geography name reference data (ONS identifiers)
CREATE TABLE geography_name_references (
    ons_code VARCHAR(20) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    geography_type VARCHAR(100) NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX idx_ods_role_refs_primary ON ods_role_references(is_primary_role_type);
CREATE INDEX idx_geo_name_refs_type ON geography_name_references(geography_type);
```

- [ ] **Step 2: Write down migration**

```sql
DROP INDEX IF EXISTS idx_geo_name_refs_type;
DROP INDEX IF EXISTS idx_ods_role_refs_primary;

DROP TABLE IF EXISTS geography_name_references;
DROP TABLE IF EXISTS practitioner_role_references;
DROP TABLE IF EXISTS ods_record_use_type_references;
DROP TABLE IF EXISTS ods_record_class_references;
DROP TABLE IF EXISTS ods_relationship_references;
DROP TABLE IF EXISTS ods_role_references;
```

- [ ] **Step 3: Commit**

```bash
git add migrations/2024122800000009_create_codesystem_tables/
git commit -m "feat: migration for ODS CodeSystem reference tables"
```

---

### Task 8: SeaORM Entities for New Tables

**Files:**
- Modify: `src/db/models.rs`

- [ ] **Step 1: Add organizations ODS columns to existing entity**

Update the `organizations` module in `src/db/models.rs` to add:

```rust
pub ods_code: Option<String>,
pub ods_status: Option<String>,
pub record_class: Option<String>,
pub record_use_type: Option<String>,
pub assigning_authority: Option<String>,
pub last_change_date: Option<NaiveDate>,
```

Also add new relations:
```rust
#[sea_orm(has_many = "super::organization_roles::Entity")]
Roles,
#[sea_orm(has_many = "super::organization_relationships::Entity")]
Relationships,
#[sea_orm(has_many = "super::organization_successions::Entity")]
Successions,
#[sea_orm(has_many = "super::organization_periods::Entity")]
Periods,
```

- [ ] **Step 2: Add SeaORM entity modules for all new tables**

Add to `src/db/models.rs`:

- `organization_roles` — (id, organization_id, unique_role_id, role_code, role_name, is_primary, status, created_at, updated_at)
- `organization_role_periods` — (id, organization_role_id, period_type, start_date, end_date)
- `organization_relationships` — (id, organization_id, unique_rel_id, relationship_type_code, relationship_type_name, status, target_ods_code, target_primary_role_id, created_at, updated_at)
- `organization_relationship_periods` — (id, organization_relationship_id, period_type, start_date, end_date)
- `organization_successions` — (id, organization_id, unique_succ_id, succession_type, target_ods_code, target_primary_role_id, legal_start_date, has_forward_succession, created_at)
- `organization_periods` — (id, organization_id, period_type, start_date, end_date)
- `postcode_geography` — (postcode PK, lsoa11, local_authority, local_authority_name, icb, icb_name, nhs_england_region, nhs_england_region_name, parliamentary_constituency, parliamentary_constituency_name, government_office_region, cancer_alliance, updated_at)
- `ods_role_references` — (role_id PK, role_name, is_primary_role_type, updated_at)
- `ods_relationship_references` — (relationship_id PK, relationship_name, updated_at)
- `ods_record_class_references` — (code PK, name, updated_at)
- `ods_record_use_type_references` — (code PK, name, updated_at)
- `practitioner_role_references` — (role_code PK, role_name, role_category, updated_at)
- `geography_name_references` — (ons_code PK, name, geography_type, updated_at)

Each entity needs: struct Model with DeriveEntityModel, Relation enum, Related impls, ActiveModelBehavior impl.

- [ ] **Step 3: Run check**

Run: `cargo check`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/db/models.rs
git commit -m "feat: add SeaORM entities for ODS tables and CodeSystem reference tables"
```

---

### Task 9: Add ODS IdentifierType and Final Integration Test

**Files:**
- Modify: `src/models/identifier.rs`

- [ ] **Step 1: Verify ODS identifier type was added in Task 4**

Run: `cargo test --lib`
Expected: all tests pass including new organization, ODS, geography, and codesystem tests

- [ ] **Step 2: Run full test suite**

Run: `cargo test --lib`
Expected: 110+ tests pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --lib`
Expected: no errors

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat: complete NHS ODS organization model with CodeSystems"
```
