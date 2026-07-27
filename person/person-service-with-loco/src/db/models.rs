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

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// Person Models
// ============================================================================

/// The `persons` table: the core person record (scalar fields only;
/// names/identifiers/etc. live in child tables).
pub mod persons {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDate, TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub birth_date: Option<TimeDate>,
        /// Tax identifier (CPF, SSN, TIN), if recorded.
        pub tax_id: Option<String>,
        /// Whether the person is deceased.
        pub deceased: bool,
        /// Date/time of death, if known.
        pub deceased_datetime: Option<TimeDateTimeWithTimeZone>,
        /// Marital-status code, if recorded.
        pub marital_status: Option<String>,
        /// Multiple-birth indicator, if recorded.
        pub multiple_birth: Option<bool>,
        /// Managing organization id (nullable FK).
        pub managing_organization_id: Option<Uuid>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// User who created the record.
        pub created_by: Option<String>,
        /// User who last updated the record.
        pub updated_by: Option<String>,
        /// Soft-delete timestamp (set when deleted).
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
        /// User who soft-deleted the record.
        pub deleted_by: Option<String>,
        /// SHA-256 over the assembled record, for out-of-band tamper
        /// detection (`crate::compliance::record_integrity`). `None` on
        /// rows written before the column existed; never back-filled,
        /// because a back-fill would certify whatever the current content
        /// happens to be — which is the claim the hash exists to test.
        pub content_hash: Option<String>,
        /// SHA-3 digest over the same pre-image as `content_hash`.
        pub content_hash_sha3: Option<String>,
    }

    /// Foreign-key relations from `persons` to its child tables and
    /// (optionally) its managing organization.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `has_many` `person_names`.
        #[sea_orm(has_many = "super::person_names::Entity")]
        PersonNames,
        /// `has_many` `person_identifiers`.
        #[sea_orm(has_many = "super::person_identifiers::Entity")]
        PersonIdentifiers,
        /// `has_many` `person_addresses`.
        #[sea_orm(has_many = "super::person_addresses::Entity")]
        PersonAddresses,
        /// `has_many` `person_contacts`.
        #[sea_orm(has_many = "super::person_contacts::Entity")]
        PersonContacts,
        /// `has_many` `person_links`.
        #[sea_orm(has_many = "super::person_links::Entity")]
        PersonLinks,
        /// `has_many` `person_match_scores`.
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
        fn to() -> RelationDef {
            Relation::PersonNames.def()
        }
    }
    impl Related<super::person_identifiers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::PersonIdentifiers.def()
        }
    }
    impl Related<super::person_addresses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::PersonAddresses.def()
        }
    }
    impl Related<super::person_contacts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::PersonContacts.def()
        }
    }
    impl Related<super::person_links::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::PersonLinks.def()
        }
    }
    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Name Models
// ============================================================================

/// The `person_names` table: primary and additional names per person.
pub mod person_names {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Identifier Models
// ============================================================================

/// The `person_identifiers` table: external IDs (MRN, SSN, TAX, …).
pub mod person_identifiers {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Address Models
// ============================================================================

/// The `person_addresses` table: physical/postal addresses per person.
pub mod person_addresses {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Contact Models
// ============================================================================

/// The `person_contacts` table: telecom contact points per person.
pub mod person_contacts {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Link Models
// ============================================================================

/// The `person_links` table: typed links between two person records
/// (e.g. `Replaces` after a merge).
pub mod person_links {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Models
// ============================================================================

/// The `organizations` table: managing/owning organizations.
pub mod organizations {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// User who created the record.
        pub created_by: Option<String>,
        /// User who last updated the record.
        pub updated_by: Option<String>,
        /// Soft-delete timestamp (set when deleted).
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
        /// User who soft-deleted the record.
        pub deleted_by: Option<String>,
    }

    /// Foreign-key relations from `organizations` to its child tables.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// `has_many` `organization_addresses`.
        #[sea_orm(has_many = "super::organization_addresses::Entity")]
        Addresses,
        /// `has_many` `organization_contacts`.
        #[sea_orm(has_many = "super::organization_contacts::Entity")]
        Contacts,
        /// `has_many` `organization_identifiers`.
        #[sea_orm(has_many = "super::organization_identifiers::Entity")]
        Identifiers,
    }

    impl Related<super::organization_addresses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Addresses.def()
        }
    }
    impl Related<super::organization_contacts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Contacts.def()
        }
    }
    impl Related<super::organization_identifiers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Identifiers.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Address Models
// ============================================================================

/// The `organization_addresses` table: addresses per organization.
pub mod organization_addresses {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Contact Models
// ============================================================================

/// The `organization_contacts` table: telecom points per organization.
pub mod organization_contacts {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Identifier Models
// ============================================================================

/// The `organization_identifiers` table: external IDs per organization.
pub mod organization_identifiers {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Match Score Models
// ============================================================================

/// The `person_match_scores` table: persisted match-score history with
/// per-component breakdown (decimals stored as `BigDecimal`).
pub mod person_match_scores {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDateTimeWithTimeZone, Uuid,
    };

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
        pub calculated_at: TimeDateTimeWithTimeZone,
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
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Audit Log Models
// ============================================================================

/// The `audit_log` table: the HIPAA-style change trail
/// (see [`AuditLogRepository`](super::audit::AuditLogRepository)).
pub mod audit_log {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EnumIter, PrimaryKeyTrait, Serialize, TimeDateTimeWithTimeZone, Uuid,
    };

    /// One audit row: action + entity + old/new JSON + request context.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// When the audited action occurred.
        pub timestamp: TimeDateTimeWithTimeZone,
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
        /// Monotonic append order. The primary key is an app-assigned
        /// UUID, which gives no insertion order, and a hash chain needs a
        /// total order — so a `BIGSERIAL` supplies it.
        pub seq: i64,
        /// Hash of the preceding chain row; `None` for the genesis row and
        /// for rows written before the chain existed.
        pub prev_hash: Option<String>,
        /// SHA-3 digest of the preceding chain row.
        pub prev_hash_sha3: Option<String>,
        /// This row's content hash — the link every successor binds to.
        pub hash: Option<String>,
        /// This row's SHA-3 digest — the third parallel chain's link.
        pub hash_sha3: Option<String>,
        /// Request/processing context (purpose-of-use, disclosure recipient).
        pub context: Option<serde_json::Value>,
        /// Whether this access was an outward **disclosure** rather than an
        /// internal access — the HIPAA §164.528 accounting distinction.
        pub disclosure: bool,
        /// When the row's content was destroyed under GDPR Art. 17.
        pub redacted_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// No outbound relations: audit rows reference entities loosely by
    /// `(entity_type, entity_id)` rather than via foreign keys.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Document Models
// ============================================================================

/// The `person_documents` table: identity documents per person.
pub mod person_documents {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        TimeDate, Uuid,
    };

    /// An identity-document row (passport, driver's license, …).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_documents")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning person id (FK).
        pub person_id: Uuid,
        /// Document type (PASCALCASE debug form, e.g. `Passport`).
        pub document_type: String,
        /// Document number.
        pub number: String,
        /// Issuing country, if recorded.
        pub issuing_country: Option<String>,
        /// Issuing authority, if recorded.
        pub issuing_authority: Option<String>,
        /// Issue date, if recorded.
        pub issue_date: Option<TimeDate>,
        /// Expiry date, if recorded.
        pub expiry_date: Option<TimeDate>,
        /// Whether the document has been verified.
        pub verified: bool,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `persons`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }
    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Emergency Contact Models
// ============================================================================

/// The `person_emergency_contacts` table: emergency contacts per person
/// (address flattened; telecom lives in `person_emergency_contact_telecom`).
pub mod person_emergency_contacts {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        Uuid,
    };

    /// An emergency-contact row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_emergency_contacts")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning person id (FK).
        pub person_id: Uuid,
        /// Contact name.
        pub name: String,
        /// Relationship (spouse, parent, …).
        pub relationship: String,
        /// Whether this is the primary emergency contact.
        pub is_primary: bool,
        /// Flattened address use type.
        pub address_use_type: Option<String>,
        /// Flattened address line 1.
        pub address_line1: Option<String>,
        /// Flattened address line 2.
        pub address_line2: Option<String>,
        /// Flattened address city.
        pub address_city: Option<String>,
        /// Flattened address state.
        pub address_state: Option<String>,
        /// Flattened address postal code.
        pub address_postal_code: Option<String>,
        /// Flattened address country.
        pub address_country: Option<String>,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `persons`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }
    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `person_emergency_contact_telecom` table: contact points per
/// emergency contact.
pub mod person_emergency_contact_telecom {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        Uuid,
    };

    /// A telecom row for one emergency contact.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_emergency_contact_telecom")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning emergency-contact id (FK).
        pub emergency_contact_id: Uuid,
        /// Contact system (`PascalCase` debug form).
        pub system: String,
        /// Contact value.
        pub value: String,
        /// Contact use/type code, if recorded.
        pub use_type: Option<String>,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent emergency contact.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `person_emergency_contacts`.
        #[sea_orm(
            belongs_to = "super::person_emergency_contacts::Entity",
            from = "Column::EmergencyContactId",
            to = "super::person_emergency_contacts::Column::Id"
        )]
        EmergencyContact,
    }
    impl Related<super::person_emergency_contacts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::EmergencyContact.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Person Photo Models
// ============================================================================

/// The `person_photos` table: photo references per person.
pub mod person_photos {
    use super::{
        ActiveModelBehavior, DeriveEntityModel, DerivePrimaryKey, DeriveRelation, Deserialize,
        EntityTrait, EnumIter, PrimaryKeyTrait, Related, RelationDef, RelationTrait, Serialize,
        Uuid,
    };

    /// A photo-reference row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "person_photos")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning person id (FK).
        pub person_id: Uuid,
        /// The photo URL / reference.
        pub url: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent person.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `persons`.
        #[sea_orm(
            belongs_to = "super::persons::Entity",
            from = "Column::PersonId",
            to = "super::persons::Column::Id"
        )]
        Person,
    }
    impl Related<super::persons::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Person.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// event_outbox
// ============================================================================

/// `SeaORM` entity for the `event_outbox` table — the transactional-outbox
/// hand-off buffer for the durable event bus (Phase 2;
/// `agents/share/event-bus.md` §3). One row is written **in the same
/// transaction** as the entity mutation, so a committed change always has
/// its event and vice versa; a relay worker (Phase 3, roadmap) drains
/// unpublished rows to Fluvio and stamps `published_at`.
pub mod event_outbox {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One persisted outbox row: a canonical envelope awaiting relay to
    /// the durable bus.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
    #[sea_orm(table_name = "event_outbox")]
    pub struct Model {
        /// Auto-increment pk; also the global relay order (`ORDER BY id`).
        #[sea_orm(primary_key)]
        pub id: i64,
        /// Envelope id — the consumer dedup key.
        #[sea_orm(unique)]
        pub event_id: Uuid,
        /// The entity name (`person`).
        pub entity: String,
        /// The record pid — the bus partition key.
        pub entity_pid: Uuid,
        /// The change kind: `created` / `updated` / `deleted` / `merged`.
        pub kind: String,
        /// When the change occurred (stamped at enqueue).
        pub occurred_at: TimeDateTimeWithTimeZone,
        /// The acting user pid, or `None` when unauthenticated.
        pub actor: Option<String>,
        /// The envelope schema version.
        pub schema_version: i32,
        /// The full canonical envelope as JSONB.
        pub payload: Json,
        /// `None` until the relay ships the row to the bus.
        pub published_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// `SeaORM` relations for the outbox entity (none defined).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// entity_links
// ============================================================================

/// `SeaORM` entity for the `entity_links` table — the cross-service
/// linking **write side** (`agents/share/cross-service-linking.md` §4.1).
/// Each row is one **outbound** edge this service originates from a
/// person; for v1 the `same_identity` (person ↔ worker) backbone edge
/// (§9). The write is optimistic — it stores the assertion and never
/// calls the target service; verification is the read-model aggregator's
/// concern. Persistence helpers (idempotent upsert, list-active,
/// bulk-list, soft-delete) live in [`crate::db::entity_links`]; edge
/// **validation** (parse `to_ref`, resolve the kind, check the endpoint
/// pair) is DB-free in `api::rest::links`.
pub mod entity_links {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One outbound cross-service edge row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "entity_links")]
    pub struct Model {
        /// Edge id (also the event's `edge_id`); application-assigned.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// This service's originating person record (its `id`).
        pub from_pid: Uuid,
        /// Edge kind token (§9 registry), e.g. `same_identity`.
        pub kind: String,
        /// The far record's `EntityRef` URN, e.g. `worker:<uuid>`.
        pub to_ref: String,
        /// Optional role label (unused by `same_identity`).
        pub role: Option<String>,
        /// Optional confidence: `1.0` operator-asserted, `<1` suggested.
        pub confidence: Option<f64>,
        /// How the edge arose: `operator` | `import` | `matcher_suggested`.
        pub provenance: String,
        /// Affiliation start (nullable — `same_identity` is not temporal).
        pub valid_from: Option<TimeDate>,
        /// Affiliation end ("former …" once past).
        pub valid_to: Option<TimeDate>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Soft-delete timestamp (a withdrawn edge); `None` while live.
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// `SeaORM` relations for the entity-links entity (none defined — the
    /// far endpoint lives in another service, referenced only by URN).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Bulk import/export job models
// ============================================================================

/// The `bulk_jobs` table: one async bulk import/export operation, drained
/// by a `bg_pg` worker (`agents/share/bulk-import-export.md` §3).
pub mod bulk_jobs {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One bulk-job row: kind + format + status, per-row counts, and the
    /// artifact references (input, result, error report).
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
    #[sea_orm(table_name = "bulk_jobs")]
    pub struct Model {
        /// Application-assigned primary key (the job id).
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// `import` or `export`.
        pub kind: String,
        /// The entity name (`person`).
        pub entity: String,
        /// File format token (`jsonl`).
        pub format: String,
        /// `queued|running|completed|completed_with_errors|failed`.
        pub status: String,
        /// Free-form job parameters (dry-run flag, export filter, …).
        pub params: Json,
        /// Total record rows seen, once known.
        pub rows_total: Option<i64>,
        /// Rows processed so far.
        pub rows_processed: i64,
        /// Rows inserted as new records.
        pub rows_created: i64,
        /// Rows upserted onto an existing record (idempotent re-import).
        pub rows_upserted: i64,
        /// Rows routed to the duplicate review queue (deferred; always 0).
        pub rows_to_review: i64,
        /// Rows that failed validation/parse/persist.
        pub rows_errored: i64,
        /// Acting user pid (bearer `sub`), if any.
        pub actor: Option<String>,
        /// Client-supplied idempotency key, if any.
        pub idempotency_key: Option<String>,
        /// Artifact reference for the uploaded source file.
        pub input_url: Option<String>,
        /// Artifact reference for the export output / import receipt.
        pub result_url: Option<String>,
        /// Artifact reference for the downloadable per-row error report.
        pub error_report_url: Option<String>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// Artifact + row TTL, if set.
        pub expires_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// `SeaORM` relations for the bulk-jobs entity (none defined).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
