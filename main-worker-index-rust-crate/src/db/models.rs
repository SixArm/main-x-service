//! Database models (SeaORM entities)
//!
//! These models are used for database operations and are separate from
//! the domain models in src/models to maintain separation of concerns.

use chrono::NaiveDate;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Worker Models
// ============================================================================

pub mod workers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "workers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub active: bool,
        pub worker_type: Option<String>,
        pub gender: String,
        pub birth_date: Option<NaiveDate>,
        pub deceased: bool,
        pub deceased_datetime: Option<DateTimeUtc>,
        pub marital_status: Option<String>,
        pub multiple_birth: Option<bool>,
        pub managing_organization_id: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        pub created_by: Option<String>,
        pub updated_by: Option<String>,
        pub deleted_at: Option<DateTimeUtc>,
        pub deleted_by: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::worker_names::Entity")]
        WorkerNames,
        #[sea_orm(has_many = "super::worker_identifiers::Entity")]
        WorkerIdentifiers,
        #[sea_orm(has_many = "super::worker_addresses::Entity")]
        WorkerAddresses,
        #[sea_orm(has_many = "super::worker_contacts::Entity")]
        WorkerContacts,
        #[sea_orm(has_many = "super::worker_links::Entity")]
        WorkerLinks,
        #[sea_orm(has_many = "super::worker_match_scores::Entity")]
        WorkerMatchScores,
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::ManagingOrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::worker_names::Entity> for Entity {
        fn to() -> RelationDef { Relation::WorkerNames.def() }
    }
    impl Related<super::worker_identifiers::Entity> for Entity {
        fn to() -> RelationDef { Relation::WorkerIdentifiers.def() }
    }
    impl Related<super::worker_addresses::Entity> for Entity {
        fn to() -> RelationDef { Relation::WorkerAddresses.def() }
    }
    impl Related<super::worker_contacts::Entity> for Entity {
        fn to() -> RelationDef { Relation::WorkerContacts.def() }
    }
    impl Related<super::worker_links::Entity> for Entity {
        fn to() -> RelationDef { Relation::WorkerLinks.def() }
    }
    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Name Models
// ============================================================================

pub mod worker_names {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_names")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub worker_id: Uuid,
        pub use_type: Option<String>,
        pub family: String,
        pub given: Vec<String>,
        pub prefix: Vec<String>,
        pub suffix: Vec<String>,
        pub is_primary: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef { Relation::Worker.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Identifier Models
// ============================================================================

pub mod worker_identifiers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_identifiers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub worker_id: Uuid,
        pub use_type: Option<String>,
        pub identifier_type: String,
        pub system: String,
        pub value: String,
        pub assigner: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef { Relation::Worker.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Address Models
// ============================================================================

pub mod worker_addresses {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_addresses")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub worker_id: Uuid,
        pub use_type: Option<String>,
        pub line1: Option<String>,
        pub line2: Option<String>,
        pub city: Option<String>,
        pub state: Option<String>,
        pub postal_code: Option<String>,
        pub country: Option<String>,
        pub is_primary: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef { Relation::Worker.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Contact Models
// ============================================================================

pub mod worker_contacts {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_contacts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub worker_id: Uuid,
        pub system: String,
        pub value: String,
        pub use_type: Option<String>,
        pub is_primary: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef { Relation::Worker.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Link Models
// ============================================================================

pub mod worker_links {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_links")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub worker_id: Uuid,
        pub other_worker_id: Uuid,
        pub link_type: String,
        pub created_at: DateTimeUtc,
        pub created_by: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef { Relation::Worker.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Models
// ============================================================================

pub mod organizations {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organizations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub active: bool,
        pub ods_code: Option<String>,
        pub ods_status: Option<String>,
        pub record_class: Option<String>,
        pub record_use_type: Option<String>,
        pub assigning_authority: Option<String>,
        pub last_change_date: Option<NaiveDate>,
        pub name: String,
        pub alias: Vec<String>,
        pub org_type: Vec<String>,
        pub part_of: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
        pub created_by: Option<String>,
        pub updated_by: Option<String>,
        pub deleted_at: Option<DateTimeUtc>,
        pub deleted_by: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(has_many = "super::organization_addresses::Entity")]
        Addresses,
        #[sea_orm(has_many = "super::organization_contacts::Entity")]
        Contacts,
        #[sea_orm(has_many = "super::organization_identifiers::Entity")]
        Identifiers,
        #[sea_orm(has_many = "super::organization_roles::Entity")]
        Roles,
        #[sea_orm(has_many = "super::organization_relationships::Entity")]
        Relationships,
        #[sea_orm(has_many = "super::organization_successions::Entity")]
        Successions,
        #[sea_orm(has_many = "super::organization_periods::Entity")]
        Periods,
    }

    impl Related<super::organization_addresses::Entity> for Entity {
        fn to() -> RelationDef { Relation::Addresses.def() }
    }
    impl Related<super::organization_contacts::Entity> for Entity {
        fn to() -> RelationDef { Relation::Contacts.def() }
    }
    impl Related<super::organization_identifiers::Entity> for Entity {
        fn to() -> RelationDef { Relation::Identifiers.def() }
    }
    impl Related<super::organization_roles::Entity> for Entity {
        fn to() -> RelationDef { Relation::Roles.def() }
    }
    impl Related<super::organization_relationships::Entity> for Entity {
        fn to() -> RelationDef { Relation::Relationships.def() }
    }
    impl Related<super::organization_successions::Entity> for Entity {
        fn to() -> RelationDef { Relation::Successions.def() }
    }
    impl Related<super::organization_periods::Entity> for Entity {
        fn to() -> RelationDef { Relation::Periods.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Address Models
// ============================================================================

pub mod organization_addresses {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_addresses")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub use_type: Option<String>,
        pub line1: Option<String>,
        pub line2: Option<String>,
        pub city: Option<String>,
        pub state: Option<String>,
        pub postal_code: Option<String>,
        pub country: Option<String>,
        pub is_primary: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Contact Models
// ============================================================================

pub mod organization_contacts {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_contacts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub system: String,
        pub value: String,
        pub use_type: Option<String>,
        pub is_primary: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Identifier Models
// ============================================================================

pub mod organization_identifiers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_identifiers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub use_type: Option<String>,
        pub identifier_type: String,
        pub system: String,
        pub value: String,
        pub assigner: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Match Score Models
// ============================================================================

pub mod worker_match_scores {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_match_scores")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub worker_id: Uuid,
        pub candidate_id: Uuid,
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub total_score: bigdecimal::BigDecimal,
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub name_score: Option<bigdecimal::BigDecimal>,
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub birth_date_score: Option<bigdecimal::BigDecimal>,
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub gender_score: Option<bigdecimal::BigDecimal>,
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub address_score: Option<bigdecimal::BigDecimal>,
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub identifier_score: Option<bigdecimal::BigDecimal>,
        pub calculated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef { Relation::Worker.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Audit Log Models
// ============================================================================

pub mod audit_log {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub timestamp: DateTimeUtc,
        pub user_id: Option<String>,
        pub action: String,
        pub entity_type: String,
        pub entity_id: Uuid,
        pub old_values: Option<serde_json::Value>,
        pub new_values: Option<serde_json::Value>,
        pub ip_address: Option<String>,
        pub user_agent: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Role Models (ODS)
// ============================================================================

pub mod organization_roles {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_roles")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub unique_role_id: i64,
        pub role_code: String,
        pub role_name: Option<String>,
        pub is_primary: bool,
        pub status: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
        #[sea_orm(has_many = "super::organization_role_periods::Entity")]
        Periods,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }
    impl Related<super::organization_role_periods::Entity> for Entity {
        fn to() -> RelationDef { Relation::Periods.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Role Period Models (ODS)
// ============================================================================

pub mod organization_role_periods {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_role_periods")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_role_id: Uuid,
        pub period_type: String,
        pub start_date: Option<NaiveDate>,
        pub end_date: Option<NaiveDate>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organization_roles::Entity",
            from = "Column::OrganizationRoleId",
            to = "super::organization_roles::Column::Id"
        )]
        OrganizationRole,
    }

    impl Related<super::organization_roles::Entity> for Entity {
        fn to() -> RelationDef { Relation::OrganizationRole.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Relationship Models (ODS)
// ============================================================================

pub mod organization_relationships {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_relationships")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub unique_rel_id: i64,
        pub relationship_type_code: String,
        pub relationship_type_name: Option<String>,
        pub status: String,
        pub target_ods_code: String,
        pub target_primary_role_id: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
        #[sea_orm(has_many = "super::organization_relationship_periods::Entity")]
        Periods,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }
    impl Related<super::organization_relationship_periods::Entity> for Entity {
        fn to() -> RelationDef { Relation::Periods.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Relationship Period Models (ODS)
// ============================================================================

pub mod organization_relationship_periods {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_relationship_periods")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_relationship_id: Uuid,
        pub period_type: String,
        pub start_date: Option<NaiveDate>,
        pub end_date: Option<NaiveDate>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organization_relationships::Entity",
            from = "Column::OrganizationRelationshipId",
            to = "super::organization_relationships::Column::Id"
        )]
        OrganizationRelationship,
    }

    impl Related<super::organization_relationships::Entity> for Entity {
        fn to() -> RelationDef { Relation::OrganizationRelationship.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Succession Models (ODS)
// ============================================================================

pub mod organization_successions {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_successions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub unique_succ_id: i64,
        pub succession_type: String,
        pub target_ods_code: String,
        pub target_primary_role_id: Option<String>,
        pub legal_start_date: Option<NaiveDate>,
        pub has_forward_succession: bool,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Period Models (ODS)
// ============================================================================

pub mod organization_periods {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_periods")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub organization_id: Uuid,
        pub period_type: String,
        pub start_date: Option<NaiveDate>,
        pub end_date: Option<NaiveDate>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Postcode Geography Models
// ============================================================================

pub mod postcode_geography {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "postcode_geography")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub postcode: String,
        pub lsoa11: Option<String>,
        pub local_authority: Option<String>,
        pub local_authority_name: Option<String>,
        pub icb: Option<String>,
        pub icb_name: Option<String>,
        pub nhs_england_region: Option<String>,
        pub nhs_england_region_name: Option<String>,
        pub parliamentary_constituency: Option<String>,
        pub parliamentary_constituency_name: Option<String>,
        pub government_office_region: Option<String>,
        pub cancer_alliance: Option<String>,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// ODS CodeSystem Reference Tables
// ============================================================================

pub mod ods_role_references {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ods_role_references")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub role_id: String,
        pub role_name: String,
        pub is_primary_role_type: bool,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod ods_relationship_references {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ods_relationship_references")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub relationship_id: String,
        pub relationship_name: String,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod ods_record_class_references {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ods_record_class_references")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub code: String,
        pub name: String,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod ods_record_use_type_references {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ods_record_use_type_references")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub code: String,
        pub name: String,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod practitioner_role_references {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "practitioner_role_references")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub role_code: String,
        pub role_name: String,
        pub role_category: Option<String>,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod geography_name_references {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "geography_name_references")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub ons_code: String,
        pub name: String,
        pub geography_type: String,
        pub updated_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
