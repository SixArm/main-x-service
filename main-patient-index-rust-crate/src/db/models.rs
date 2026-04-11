//! Database models (SeaORM entities)
//!
//! These models are used for database operations and are separate from
//! the domain models in src/models to maintain separation of concerns.

use chrono::NaiveDate;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Patient Models
// ============================================================================

pub mod patients {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "patients")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub active: bool,
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
        #[sea_orm(has_many = "super::patient_names::Entity")]
        PatientNames,
        #[sea_orm(has_many = "super::patient_identifiers::Entity")]
        PatientIdentifiers,
        #[sea_orm(has_many = "super::patient_addresses::Entity")]
        PatientAddresses,
        #[sea_orm(has_many = "super::patient_contacts::Entity")]
        PatientContacts,
        #[sea_orm(has_many = "super::patient_links::Entity")]
        PatientLinks,
        #[sea_orm(has_many = "super::patient_match_scores::Entity")]
        PatientMatchScores,
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::ManagingOrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::patient_names::Entity> for Entity {
        fn to() -> RelationDef { Relation::PatientNames.def() }
    }
    impl Related<super::patient_identifiers::Entity> for Entity {
        fn to() -> RelationDef { Relation::PatientIdentifiers.def() }
    }
    impl Related<super::patient_addresses::Entity> for Entity {
        fn to() -> RelationDef { Relation::PatientAddresses.def() }
    }
    impl Related<super::patient_contacts::Entity> for Entity {
        fn to() -> RelationDef { Relation::PatientContacts.def() }
    }
    impl Related<super::patient_links::Entity> for Entity {
        fn to() -> RelationDef { Relation::PatientLinks.def() }
    }
    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Patient Name Models
// ============================================================================

pub mod patient_names {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "patient_names")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub patient_id: Uuid,
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
            belongs_to = "super::patients::Entity",
            from = "Column::PatientId",
            to = "super::patients::Column::Id"
        )]
        Patient,
    }

    impl Related<super::patients::Entity> for Entity {
        fn to() -> RelationDef { Relation::Patient.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Patient Identifier Models
// ============================================================================

pub mod patient_identifiers {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "patient_identifiers")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub patient_id: Uuid,
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
            belongs_to = "super::patients::Entity",
            from = "Column::PatientId",
            to = "super::patients::Column::Id"
        )]
        Patient,
    }

    impl Related<super::patients::Entity> for Entity {
        fn to() -> RelationDef { Relation::Patient.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Patient Address Models
// ============================================================================

pub mod patient_addresses {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "patient_addresses")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub patient_id: Uuid,
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
            belongs_to = "super::patients::Entity",
            from = "Column::PatientId",
            to = "super::patients::Column::Id"
        )]
        Patient,
    }

    impl Related<super::patients::Entity> for Entity {
        fn to() -> RelationDef { Relation::Patient.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Patient Contact Models
// ============================================================================

pub mod patient_contacts {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "patient_contacts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub patient_id: Uuid,
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
            belongs_to = "super::patients::Entity",
            from = "Column::PatientId",
            to = "super::patients::Column::Id"
        )]
        Patient,
    }

    impl Related<super::patients::Entity> for Entity {
        fn to() -> RelationDef { Relation::Patient.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Patient Link Models
// ============================================================================

pub mod patient_links {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "patient_links")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub patient_id: Uuid,
        pub other_patient_id: Uuid,
        pub link_type: String,
        pub created_at: DateTimeUtc,
        pub created_by: Option<String>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        #[sea_orm(
            belongs_to = "super::patients::Entity",
            from = "Column::PatientId",
            to = "super::patients::Column::Id"
        )]
        Patient,
    }

    impl Related<super::patients::Entity> for Entity {
        fn to() -> RelationDef { Relation::Patient.def() }
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
// Patient Match Score Models
// ============================================================================

pub mod patient_match_scores {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "patient_match_scores")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub patient_id: Uuid,
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
            belongs_to = "super::patients::Entity",
            from = "Column::PatientId",
            to = "super::patients::Column::Id"
        )]
        Patient,
    }

    impl Related<super::patients::Entity> for Entity {
        fn to() -> RelationDef { Relation::Patient.def() }
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
