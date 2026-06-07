//! SeaORM entity definitions — one submodule per PostgreSQL table.
//!
//! These persistence entities are deliberately kept separate from the
//! domain models in [`crate::models`]: the domain types are the public,
//! API-facing shapes, while these mirror the normalized relational
//! schema (a person's names, identifiers, addresses, contacts, and links
//! live in child tables joined by `person_id`). Each submodule uses
//! SeaORM's [`DeriveEntityModel`](sea_orm::DeriveEntityModel) macro, which generates the companion
//! `Entity`, `Column`, `PrimaryKey`, and `ActiveModel` types from the
//! `Model` struct; `Relation` enums + `Related` impls wire up the
//! foreign keys. UUID primary keys are application-assigned
//! (`auto_increment = false`).
//!
//! Tables: [`persons`](crate::db::models::persons), [`person_names`](crate::db::models::person_names), [`person_identifiers`](crate::db::models::person_identifiers),
//! [`person_addresses`](crate::db::models::person_addresses), [`person_contacts`](crate::db::models::person_contacts), [`person_links`](crate::db::models::person_links),
//! [`organizations`](crate::db::models::organizations), [`organization_addresses`](crate::db::models::organization_addresses),
//! [`organization_contacts`](crate::db::models::organization_contacts), [`organization_identifiers`](crate::db::models::organization_identifiers),
//! [`person_match_scores`](crate::db::models::person_match_scores), and [`audit_log`](crate::db::models::audit_log).

use chrono::NaiveDate;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Person Models
// ============================================================================

/// The `persons` table: the core person record (scalar fields only;
/// names/identifiers/etc. live in child tables).
pub mod persons {
    use super::*;

    /// A row in `persons`. Soft-delete is via `deleted_at`/`deleted_by`.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "persons")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Whether the record is active.
        pub active: bool,
        /// Gender code (lowercased FHIR value).
        pub gender: String,
        /// Date of birth, if known.
        pub birth_date: Option<NaiveDate>,
        /// Whether the person is deceased.
        pub deceased: bool,
        /// Date/time of death, if known.
        pub deceased_datetime: Option<DateTimeUtc>,
        /// Marital-status code, if recorded.
        pub marital_status: Option<String>,
        /// Multiple-birth indicator, if recorded.
        pub multiple_birth: Option<bool>,
        /// Managing organization id (nullable FK).
        pub managing_organization_id: Option<Uuid>,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
        /// User who created the record.
        pub created_by: Option<String>,
        /// User who last updated the record.
        pub updated_by: Option<String>,
        /// Soft-delete timestamp (set when deleted).
        pub deleted_at: Option<DateTimeUtc>,
        /// User who soft-deleted the record.
        pub deleted_by: Option<String>,
    }

    /// Foreign-key relations from `persons` to its child tables and
    /// (optionally) its managing organization.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `has_many` person_names.
        #[sea_orm(has_many = "super::person_names::Entity")]
        PersonNames,
        /// `has_many` person_identifiers.
        #[sea_orm(has_many = "super::person_identifiers::Entity")]
        PersonIdentifiers,
        /// `has_many` person_addresses.
        #[sea_orm(has_many = "super::person_addresses::Entity")]
        PersonAddresses,
        /// `has_many` person_contacts.
        #[sea_orm(has_many = "super::person_contacts::Entity")]
        PersonContacts,
        /// `has_many` person_links.
        #[sea_orm(has_many = "super::person_links::Entity")]
        PersonLinks,
        /// `has_many` person_match_scores.
        #[sea_orm(has_many = "super::person_match_scores::Entity")]
        PersonMatchScores,
        /// `belongs_to` the managing organization (nullable FK).
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::ManagingOrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::person_names::Entity> for Entity {
        fn to() -> RelationDef { Relation::PersonNames.def() }
    }
    impl Related<super::person_identifiers::Entity> for Entity {
        fn to() -> RelationDef { Relation::PersonIdentifiers.def() }
    }
    impl Related<super::person_addresses::Entity> for Entity {
        fn to() -> RelationDef { Relation::PersonAddresses.def() }
    }
    impl Related<super::person_contacts::Entity> for Entity {
        fn to() -> RelationDef { Relation::PersonContacts.def() }
    }
    impl Related<super::person_links::Entity> for Entity {
        fn to() -> RelationDef { Relation::PersonLinks.def() }
    }
    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef { Relation::Organization.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Name Models
// ============================================================================

/// The `person_names` table: primary and additional names per person.
pub mod person_names {
    use super::*;

    /// A name row; `is_primary` flags the person's main [`HumanName`](crate::models::person::HumanName).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_names")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning person id (FK).
        pub person_id: Uuid,
        /// Name use/type code, if recorded.
        pub use_type: Option<String>,
        /// Family (last) name.
        pub family: String,
        /// Given (first/middle) names.
        pub given: Vec<String>,
        /// Name prefixes (e.g. Dr., Mr.).
        pub prefix: Vec<String>,
        /// Name suffixes (e.g. Jr., III).
        pub suffix: Vec<String>,
        /// Whether this is the person's primary name.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
    }

    /// Foreign-key relation back to the owning person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent person via `person_id`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }

    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef { Relation::Person.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Identifier Models
// ============================================================================

/// The `person_identifiers` table: external IDs (MRN, SSN, TAX, …).
pub mod person_identifiers {
    use super::*;

    /// An identifier row (type + system + value) for one person.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_identifiers")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning person id (FK).
        pub person_id: Uuid,
        /// Identifier use/type code, if recorded.
        pub use_type: Option<String>,
        /// Identifier type (MRN, SSN, TAX, …).
        pub identifier_type: String,
        /// Identifier system URI.
        pub system: String,
        /// Identifier value.
        pub value: String,
        /// Assigning authority, if recorded.
        pub assigner: Option<String>,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
    }

    /// Foreign-key relation back to the owning person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent person via `person_id`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }

    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef { Relation::Person.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Address Models
// ============================================================================

/// The `person_addresses` table: physical/postal addresses per person.
pub mod person_addresses {
    use super::*;

    /// An address row; `is_primary` flags the person's main address.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_addresses")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning person id (FK).
        pub person_id: Uuid,
        /// Address use/type code, if recorded.
        pub use_type: Option<String>,
        /// Street address line 1.
        pub line1: Option<String>,
        /// Street address line 2.
        pub line2: Option<String>,
        /// City / locality.
        pub city: Option<String>,
        /// State / region.
        pub state: Option<String>,
        /// Postal / ZIP code.
        pub postal_code: Option<String>,
        /// Country code.
        pub country: Option<String>,
        /// Whether this is the person's primary address.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
    }

    /// Foreign-key relation back to the owning person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent person via `person_id`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }

    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef { Relation::Person.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Contact Models
// ============================================================================

/// The `person_contacts` table: telecom contact points per person.
pub mod person_contacts {
    use super::*;

    /// A contact-point row (system + value) for one person.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_contacts")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning person id (FK).
        pub person_id: Uuid,
        /// Contact system (phone, email, fax, …).
        pub system: String,
        /// Contact value.
        pub value: String,
        /// Contact use/type code, if recorded.
        pub use_type: Option<String>,
        /// Whether this is the person's primary contact.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
    }

    /// Foreign-key relation back to the owning person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent person via `person_id`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }

    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef { Relation::Person.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Link Models
// ============================================================================

/// The `person_links` table: typed links between two person records
/// (e.g. `Replaces` after a merge).
pub mod person_links {
    use super::*;

    /// A link row from `person_id` to `other_person_id` with a type.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_links")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning person id (FK), the link source.
        pub person_id: Uuid,
        /// The linked-to person id.
        pub other_person_id: Uuid,
        /// Link type (Replaces, Refer, Seealso, …).
        pub link_type: String,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// User who created the link.
        pub created_by: Option<String>,
    }

    /// Foreign-key relation back to the owning person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent person via `person_id`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }

    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef { Relation::Person.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Models
// ============================================================================

/// The `organizations` table: managing/owning organizations.
pub mod organizations {
    use super::*;

    /// An organization row; `part_of` is a self-referential parent FK.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organizations")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Whether the organization is active.
        pub active: bool,
        /// Organization name.
        pub name: String,
        /// Alternative names / aliases.
        pub alias: Vec<String>,
        /// Organization type codes.
        pub org_type: Vec<String>,
        /// Parent organization id (self-referential FK).
        pub part_of: Option<Uuid>,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
        /// User who created the record.
        pub created_by: Option<String>,
        /// User who last updated the record.
        pub updated_by: Option<String>,
        /// Soft-delete timestamp (set when deleted).
        pub deleted_at: Option<DateTimeUtc>,
        /// User who soft-deleted the record.
        pub deleted_by: Option<String>,
    }

    /// Foreign-key relations from `organizations` to its child tables.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `has_many` organization_addresses.
        #[sea_orm(has_many = "super::organization_addresses::Entity")]
        Addresses,
        /// `has_many` organization_contacts.
        #[sea_orm(has_many = "super::organization_contacts::Entity")]
        Contacts,
        /// `has_many` organization_identifiers.
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

/// The `organization_addresses` table: addresses per organization.
pub mod organization_addresses {
    use super::*;

    /// An address row for one organization.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_addresses")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization id (FK).
        pub organization_id: Uuid,
        /// Address use/type code, if recorded.
        pub use_type: Option<String>,
        /// Street address line 1.
        pub line1: Option<String>,
        /// Street address line 2.
        pub line2: Option<String>,
        /// City / locality.
        pub city: Option<String>,
        /// State / region.
        pub state: Option<String>,
        /// Postal / ZIP code.
        pub postal_code: Option<String>,
        /// Country code.
        pub country: Option<String>,
        /// Whether this is the organization's primary address.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
    }

    /// Foreign-key relation back to the owning organization.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent organization via `organization_id`.
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

/// The `organization_contacts` table: telecom points per organization.
pub mod organization_contacts {
    use super::*;

    /// A contact-point row for one organization.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_contacts")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization id (FK).
        pub organization_id: Uuid,
        /// Contact system (phone, email, fax, …).
        pub system: String,
        /// Contact value.
        pub value: String,
        /// Contact use/type code, if recorded.
        pub use_type: Option<String>,
        /// Whether this is the organization's primary contact.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
    }

    /// Foreign-key relation back to the owning organization.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent organization via `organization_id`.
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

/// The `organization_identifiers` table: external IDs per organization.
pub mod organization_identifiers {
    use super::*;

    /// An identifier row (type + system + value) for one organization.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_identifiers")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization id (FK).
        pub organization_id: Uuid,
        /// Identifier use/type code, if recorded.
        pub use_type: Option<String>,
        /// Identifier type (MRN, SSN, TAX, …).
        pub identifier_type: String,
        /// Identifier system URI.
        pub system: String,
        /// Identifier value.
        pub value: String,
        /// Assigning authority, if recorded.
        pub assigner: Option<String>,
        /// Creation timestamp.
        pub created_at: DateTimeUtc,
        /// Last-update timestamp.
        pub updated_at: DateTimeUtc,
    }

    /// Foreign-key relation back to the owning organization.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent organization via `organization_id`.
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
// Person Match Score Models
// ============================================================================

/// The `person_match_scores` table: persisted match-score history with
/// per-component breakdown (decimals stored as `BigDecimal`).
pub mod person_match_scores {
    use super::*;

    /// A scored person/candidate pair with component sub-scores.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_match_scores")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// The scored person id (FK).
        pub person_id: Uuid,
        /// The candidate person id compared against.
        pub candidate_id: Uuid,
        /// Overall match score in `[0, 1]`.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub total_score: bigdecimal::BigDecimal,
        /// Name component sub-score.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub name_score: Option<bigdecimal::BigDecimal>,
        /// Birth-date component sub-score.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub birth_date_score: Option<bigdecimal::BigDecimal>,
        /// Gender component sub-score.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub gender_score: Option<bigdecimal::BigDecimal>,
        /// Address component sub-score.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub address_score: Option<bigdecimal::BigDecimal>,
        /// Identifier component sub-score.
        #[sea_orm(column_type = "Decimal(Some((10, 6)))")]
        pub identifier_score: Option<bigdecimal::BigDecimal>,
        /// When the score was computed.
        pub calculated_at: DateTimeUtc,
    }

    /// Foreign-key relation back to the owning person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `belongs_to` the parent person via `person_id`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }

    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef { Relation::Person.def() }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Audit Log Models
// ============================================================================

/// The `audit_log` table: the HIPAA-style change trail
/// (see [`AuditLogRepository`](super::audit::AuditLogRepository)).
pub mod audit_log {
    use super::*;

    /// One audit row: action + entity + old/new JSON + request context.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// When the audited action occurred.
        pub timestamp: DateTimeUtc,
        /// Acting user id, if known.
        pub user_id: Option<String>,
        /// Action performed (create, update, delete, …).
        pub action: String,
        /// Entity type the action targeted.
        pub entity_type: String,
        /// Id of the affected entity.
        pub entity_id: Uuid,
        /// Pre-change values as JSON, if applicable.
        pub old_values: Option<serde_json::Value>,
        /// Post-change values as JSON, if applicable.
        pub new_values: Option<serde_json::Value>,
        /// Request IP address, if captured.
        pub ip_address: Option<String>,
        /// Request user-agent string, if captured.
        pub user_agent: Option<String>,
    }

    /// No outbound relations: audit rows reference entities loosely by
    /// `(entity_type, entity_id)` rather than via foreign keys.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
