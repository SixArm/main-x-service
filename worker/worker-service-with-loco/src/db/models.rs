//! SeaORM entity definitions mapping PostgreSQL tables to Rust structs.
//!
//! Each submodule corresponds to one table and follows SeaORM's convention:
//! a `Model` (a row), an auto-derived `ActiveModel`/`Entity`/`Column`, a
//! `Relation` enum describing foreign keys, and `Related` impls for joins.
//! These persistence entities are deliberately kept separate from the rich
//! domain models in [`crate::models`]; repository code in
//! [`crate::db::repositories`] converts between the two.
//!
//! Each `Model` field maps 1:1 to a SQL column of the same name; `Option<T>`
//! marks a nullable column, and `Uuid` foreign-key fields name the parent row
//! they reference. Timestamp columns use the `time` crate's
//! `OffsetDateTime` (aliased `TimeDateTimeWithTimeZone` in SeaORM's prelude)
//! and date columns use `time::Date` (`TimeDate`), since SeaORM 1.1 lacks
//! native `chrono` interop here; the [`crate::db::convert`] helpers bridge to
//! the domain `chrono` types. The authoritative column reference is
//! `AGENTS/models.md`.
//!
//! Soft-delete convention: tables carrying `deleted_at` / `deleted_by` are
//! never physically deleted by the repository; a non-null `deleted_at` marks
//! the row as logically removed and queries filter it out.

use serde::{Deserialize, Serialize};

// `DeriveEntityModel` expands each `Model` into the full SeaORM entity set
// (`Entity`, `Column`, `PrimaryKey`, `ActiveModel`). `Serialize`/`Deserialize`
// let rows cross the API and audit-JSON boundaries directly. `#[sea_orm(...)]`
// attributes carry table/column metadata; a `///` doc placed *above* such an
// attribute is safe because doc comments desugar to `#[doc = ...]` attributes
// that the derive ignores.

// ============================================================================
// Worker Models
// ============================================================================

/// Core `workers` table: one row per worker identity record (demographics,
/// status flags, soft-delete and audit columns).
pub mod workers {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `workers` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "workers")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Whether the record is active.
        pub active: bool,
        /// Worker type code.
        pub worker_type: Option<String>,
        /// Administrative gender code.
        pub gender: String,
        /// Date of birth.
        pub birth_date: Option<TimeDate>,
        /// Tax identifier (CPF, SSN, TIN), if recorded.
        pub tax_id: Option<String>,
        /// Whether the worker is deceased.
        pub deceased: bool,
        /// Date/time of death.
        pub deceased_datetime: Option<TimeDateTimeWithTimeZone>,
        /// Marital status code.
        pub marital_status: Option<String>,
        /// Multiple-birth indicator.
        pub multiple_birth: Option<bool>,
        /// Managing organization foreign key (nullable; references
        /// `organizations.id`).
        pub managing_organization_id: Option<Uuid>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// Actor who created the row.
        pub created_by: Option<String>,
        /// Actor who last updated the row.
        pub updated_by: Option<String>,
        // Soft-delete pair: a non-null `deleted_at` flags the row as logically
        // removed; reads filter on `deleted_at IS NULL` (see the repository's
        // `get_by_id` / `list_active`).
        /// Soft-delete timestamp (non-null once the worker is soft-deleted).
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
        /// Actor who soft-deleted the row.
        pub deleted_by: Option<String>,
        /// SHA-256 over the assembled record, for out-of-band tamper
        /// detection (`crate::compliance::record_integrity`). `None` on
        /// rows written before the column existed; never back-filled,
        /// because a back-fill would certify whatever the current content
        /// happens to be — which is the claim the hash exists to test.
        pub content_hash: Option<String>,
        /// BLAKE3 digest over the same pre-image as `content_hash`.
        /// `None` on rows written before the second algorithm was
        /// adopted; never back-filled.
        pub content_hash_blake3: Option<String>,
    }

    /// Foreign-key relations for this entity. Workers own several child
    /// collections (`has_many`) and reference one managing organization
    /// (`belongs_to`).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Has many `worker_names`.
        #[sea_orm(has_many = "super::worker_names::Entity")]
        WorkerNames,
        /// Has many `worker_identifiers`.
        #[sea_orm(has_many = "super::worker_identifiers::Entity")]
        WorkerIdentifiers,
        /// Has many `worker_addresses`.
        #[sea_orm(has_many = "super::worker_addresses::Entity")]
        WorkerAddresses,
        /// Has many `worker_contacts`.
        #[sea_orm(has_many = "super::worker_contacts::Entity")]
        WorkerContacts,
        /// Has many `worker_links`.
        #[sea_orm(has_many = "super::worker_links::Entity")]
        WorkerLinks,
        /// Has many `worker_match_scores`.
        #[sea_orm(has_many = "super::worker_match_scores::Entity")]
        WorkerMatchScores,
        /// Belongs to the managing `organizations` row.
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::ManagingOrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
    }

    impl Related<super::worker_names::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::WorkerNames.def()
        }
    }
    impl Related<super::worker_identifiers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::WorkerIdentifiers.def()
        }
    }
    impl Related<super::worker_addresses::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::WorkerAddresses.def()
        }
    }
    impl Related<super::worker_contacts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::WorkerContacts.def()
        }
    }
    impl Related<super::worker_links::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::WorkerLinks.def()
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
// Worker Name Models
// ============================================================================

/// `worker_names` table: a worker's primary and additional names (one row per
/// name, with `is_primary` flagging the canonical one).
pub mod worker_names {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `worker_names` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_names")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning worker foreign key.
        pub worker_id: Uuid,
        /// Name use/purpose code.
        pub use_type: Option<String>,
        /// Family (last) name.
        pub family: String,
        /// Given (first/middle) names.
        pub given: Vec<String>,
        /// Name prefixes.
        pub prefix: Vec<String>,
        /// Name suffixes.
        pub suffix: Vec<String>,
        /// Whether this is the primary name.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `workers` row.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Identifier Models
// ============================================================================

/// `worker_identifiers` table: external identifiers for a worker (type +
/// system + value), e.g. MRN, SSN, national IDs.
pub mod worker_identifiers {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `worker_identifiers` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_identifiers")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning worker foreign key.
        pub worker_id: Uuid,
        /// Identifier use/purpose code.
        pub use_type: Option<String>,
        /// Identifier type code (MRN, SSN, …).
        pub identifier_type: String,
        /// Identifier namespace/system URI.
        pub system: String,
        /// Identifier value.
        pub value: String,
        /// Assigning authority.
        pub assigner: Option<String>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `workers` row.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Address Models
// ============================================================================

/// `worker_addresses` table: a worker's physical addresses (one row each).
pub mod worker_addresses {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `worker_addresses` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_addresses")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning worker foreign key.
        pub worker_id: Uuid,
        /// Address use/purpose code.
        pub use_type: Option<String>,
        /// Street address line 1.
        pub line1: Option<String>,
        /// Street address line 2.
        pub line2: Option<String>,
        /// City.
        pub city: Option<String>,
        /// State or province.
        pub state: Option<String>,
        /// Postal/ZIP code.
        pub postal_code: Option<String>,
        /// Country code or name.
        pub country: Option<String>,
        /// Whether this is the primary address.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `workers` row.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Contact Models
// ============================================================================

/// `worker_contacts` table: a worker's contact points (phone, email, fax).
pub mod worker_contacts {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `worker_contacts` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_contacts")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning worker foreign key.
        pub worker_id: Uuid,
        /// Telecom system code (phone, email, …).
        pub system: String,
        /// Contact value.
        pub value: String,
        /// Contact use/purpose code.
        pub use_type: Option<String>,
        /// Whether this is the primary contact.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `workers` row.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Link Models
// ============================================================================

/// `worker_links` table: directed links between workers (e.g. `Replaces`,
/// `ReplacedBy`) used by merge and aliasing.
pub mod worker_links {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `worker_links` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_links")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Source worker foreign key.
        pub worker_id: Uuid,
        /// Target worker foreign key.
        pub other_worker_id: Uuid,
        /// Link type code (Replaces, `ReplacedBy`, …).
        pub link_type: String,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Actor who created the link.
        pub created_by: Option<String>,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `workers` row.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Models
// ============================================================================

/// `organizations` table: managing organizations (with NHS ODS fields), one
/// row per organization.
pub mod organizations {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organizations` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organizations")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Whether the record is active.
        pub active: bool,
        /// NHS ODS organisation code.
        pub ods_code: Option<String>,
        /// ODS status code.
        pub ods_status: Option<String>,
        /// ODS record class.
        pub record_class: Option<String>,
        /// ODS record use type.
        pub record_use_type: Option<String>,
        /// Assigning authority.
        pub assigning_authority: Option<String>,
        /// Date of last ODS change.
        pub last_change_date: Option<TimeDate>,
        /// Organization name.
        pub name: String,
        /// Alternative names.
        pub alias: Vec<String>,
        /// Organization type codes.
        pub org_type: Vec<String>,
        /// Parent organization foreign key.
        pub part_of: Option<Uuid>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// Actor who created the row.
        pub created_by: Option<String>,
        /// Actor who last updated the row.
        pub updated_by: Option<String>,
        /// Soft-delete timestamp.
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
        /// Actor who soft-deleted the row.
        pub deleted_by: Option<String>,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Has many `organization_addresses`.
        #[sea_orm(has_many = "super::organization_addresses::Entity")]
        Addresses,
        /// Has many `organization_contacts`.
        #[sea_orm(has_many = "super::organization_contacts::Entity")]
        Contacts,
        /// Has many `organization_identifiers`.
        #[sea_orm(has_many = "super::organization_identifiers::Entity")]
        Identifiers,
        /// Has many `organization_roles`.
        #[sea_orm(has_many = "super::organization_roles::Entity")]
        Roles,
        /// Has many `organization_relationships`.
        #[sea_orm(has_many = "super::organization_relationships::Entity")]
        Relationships,
        /// Has many `organization_successions`.
        #[sea_orm(has_many = "super::organization_successions::Entity")]
        Successions,
        /// Has many `organization_periods`.
        #[sea_orm(has_many = "super::organization_periods::Entity")]
        Periods,
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
    impl Related<super::organization_roles::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Roles.def()
        }
    }
    impl Related<super::organization_relationships::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Relationships.def()
        }
    }
    impl Related<super::organization_successions::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Successions.def()
        }
    }
    impl Related<super::organization_periods::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Periods.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Address Models
// ============================================================================

/// `organization_addresses` table: an organization's physical addresses.
pub mod organization_addresses {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_addresses` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_addresses")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization foreign key.
        pub organization_id: Uuid,
        /// Address use/purpose code.
        pub use_type: Option<String>,
        /// Street address line 1.
        pub line1: Option<String>,
        /// Street address line 2.
        pub line2: Option<String>,
        /// City.
        pub city: Option<String>,
        /// State or province.
        pub state: Option<String>,
        /// Postal/ZIP code.
        pub postal_code: Option<String>,
        /// Country code or name.
        pub country: Option<String>,
        /// Whether this is the primary address.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organizations` row.
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

/// `organization_contacts` table: an organization's contact points.
pub mod organization_contacts {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_contacts` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_contacts")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization foreign key.
        pub organization_id: Uuid,
        /// Telecom system code (phone, email, …).
        pub system: String,
        /// Contact value.
        pub value: String,
        /// Contact use/purpose code.
        pub use_type: Option<String>,
        /// Whether this is the primary contact.
        pub is_primary: bool,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organizations` row.
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

/// `organization_identifiers` table: external identifiers for an organization.
pub mod organization_identifiers {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_identifiers` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_identifiers")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization foreign key.
        pub organization_id: Uuid,
        /// Identifier use/purpose code.
        pub use_type: Option<String>,
        /// Identifier type code.
        pub identifier_type: String,
        /// Identifier namespace/system URI.
        pub system: String,
        /// Identifier value.
        pub value: String,
        /// Assigning authority.
        pub assigner: Option<String>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organizations` row.
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
// Worker Match Score Models
// ============================================================================

/// `worker_match_scores` table: persisted match-score history between a worker
/// and a candidate, with per-component sub-scores as decimals.
pub mod worker_match_scores {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `worker_match_scores` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_match_scores")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Subject worker foreign key.
        pub worker_id: Uuid,
        /// Candidate worker foreign key.
        pub candidate_id: Uuid,
        // Scores are persisted as fixed NUMERIC(10,6) (via `BigDecimal`) rather
        // than floating point, so a stored match score round-trips exactly.
        /// Overall match score.
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
        /// When the score was calculated.
        pub calculated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `workers` row.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }

    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Audit Log Models
// ============================================================================

/// `audit_log` table: HIPAA-style change trail (action, entity, old/new JSON,
/// actor metadata). Written via [`crate::db::audit::AuditLogRepository`].
pub mod audit_log {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `audit_log` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// When the action occurred.
        pub timestamp: TimeDateTimeWithTimeZone,
        /// Acting user ID.
        pub user_id: Option<String>,
        /// Action performed (create, update, …).
        pub action: String,
        /// Entity type acted upon.
        pub entity_type: String,
        /// Entity ID acted upon.
        pub entity_id: Uuid,
        // old/new are stored as JSONB snapshots of the serialized domain record:
        // `old_values` is null on CREATE, `new_values` is null on DELETE, and
        // both are populated on UPDATE to form the before/after diff.
        /// Previous values as JSON (null for a CREATE action).
        pub old_values: Option<serde_json::Value>,
        /// New values as JSON (null for a DELETE action).
        pub new_values: Option<serde_json::Value>,
        /// Originating IP address.
        pub ip_address: Option<String>,
        /// Originating user agent.
        pub user_agent: Option<String>,
        /// Monotonic append order. The primary key is an app-assigned
        /// UUID, which gives no insertion order, and a hash chain needs a
        /// total order — so a `BIGSERIAL` supplies it.
        pub seq: i64,
        /// Hash of the preceding chain row; `None` for the genesis row and
        /// for rows written before the chain existed.
        pub prev_hash: Option<String>,
        /// BLAKE3 digest of the preceding chain row (`None` for the
        /// genesis row, or a row predating the second algorithm).
        pub prev_hash_blake3: Option<String>,
        /// This row's content hash — the link every successor binds to.
        pub hash: Option<String>,
        /// This row's BLAKE3 digest — the parallel chain's link. `None`
        /// on rows written before the second algorithm was adopted.
        pub hash_blake3: Option<String>,
        /// Request/processing context (purpose-of-use, disclosure recipient).
        pub context: Option<serde_json::Value>,
        /// Whether this access was an outward **disclosure** rather than an
        /// internal access — the HIPAA §164.528 accounting distinction.
        pub disclosure: bool,
        /// When the row's content was destroyed under GDPR Art. 17.
        pub redacted_at: Option<TimeDateTimeWithTimeZone>,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Role Models (ODS)
// ============================================================================

/// `organization_roles` table (NHS ODS): roles an organization holds, each
/// with status and an optional set of validity periods.
pub mod organization_roles {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_roles` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_roles")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization foreign key.
        pub organization_id: Uuid,
        /// ODS unique role identifier.
        pub unique_role_id: i64,
        /// ODS role code.
        pub role_code: String,
        /// Human-readable role name.
        pub role_name: Option<String>,
        /// Whether this is the primary role.
        pub is_primary: bool,
        /// Role status code.
        pub status: String,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organizations` row.
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
        /// Has many `organization_role_periods`.
        #[sea_orm(has_many = "super::organization_role_periods::Entity")]
        Periods,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }
    impl Related<super::organization_role_periods::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Periods.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Role Period Models (ODS)
// ============================================================================

/// `organization_role_periods` table (NHS ODS): validity periods for an
/// organization role.
pub mod organization_role_periods {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_role_periods` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_role_periods")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization-role foreign key.
        pub organization_role_id: Uuid,
        /// Period type code.
        pub period_type: String,
        /// Period start date.
        pub start_date: Option<TimeDate>,
        /// Period end date.
        pub end_date: Option<TimeDate>,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organization_roles` row.
        #[sea_orm(
            belongs_to = "super::organization_roles::Entity",
            from = "Column::OrganizationRoleId",
            to = "super::organization_roles::Column::Id"
        )]
        OrganizationRole,
    }

    impl Related<super::organization_roles::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::OrganizationRole.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Relationship Models (ODS)
// ============================================================================

/// `organization_relationships` table (NHS ODS): typed relationships between
/// organizations (e.g. "is commissioned by"), each with validity periods.
pub mod organization_relationships {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_relationships` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_relationships")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization foreign key.
        pub organization_id: Uuid,
        /// ODS unique relationship identifier.
        pub unique_rel_id: i64,
        /// Relationship type code.
        pub relationship_type_code: String,
        /// Human-readable relationship type name.
        pub relationship_type_name: Option<String>,
        /// Relationship status code.
        pub status: String,
        /// Target organization ODS code.
        pub target_ods_code: String,
        /// Target organization primary role ID.
        pub target_primary_role_id: Option<String>,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organizations` row.
        #[sea_orm(
            belongs_to = "super::organizations::Entity",
            from = "Column::OrganizationId",
            to = "super::organizations::Column::Id"
        )]
        Organization,
        /// Has many `organization_relationship_periods`.
        #[sea_orm(has_many = "super::organization_relationship_periods::Entity")]
        Periods,
    }

    impl Related<super::organizations::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Organization.def()
        }
    }
    impl Related<super::organization_relationship_periods::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Periods.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Relationship Period Models (ODS)
// ============================================================================

/// `organization_relationship_periods` table (NHS ODS): validity periods for
/// an organization relationship.
pub mod organization_relationship_periods {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_relationship_periods` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_relationship_periods")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization-relationship foreign key.
        pub organization_relationship_id: Uuid,
        /// Period type code.
        pub period_type: String,
        /// Period start date.
        pub start_date: Option<TimeDate>,
        /// Period end date.
        pub end_date: Option<TimeDate>,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organization_relationships` row.
        #[sea_orm(
            belongs_to = "super::organization_relationships::Entity",
            from = "Column::OrganizationRelationshipId",
            to = "super::organization_relationships::Column::Id"
        )]
        OrganizationRelationship,
    }

    impl Related<super::organization_relationships::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::OrganizationRelationship.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Organization Succession Models (ODS)
// ============================================================================

/// `organization_successions` table (NHS ODS): predecessor/successor links
/// recording organizational mergers and splits over time.
pub mod organization_successions {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_successions` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_successions")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization foreign key.
        pub organization_id: Uuid,
        /// ODS unique succession identifier.
        pub unique_succ_id: i64,
        /// Succession type code (predecessor/successor).
        pub succession_type: String,
        /// Counterpart organization ODS code.
        pub target_ods_code: String,
        /// Counterpart organization primary role ID.
        pub target_primary_role_id: Option<String>,
        /// Legal start date of the succession.
        pub legal_start_date: Option<TimeDate>,
        /// Whether this is a forward succession.
        pub has_forward_succession: bool,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organizations` row.
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
// Organization Period Models (ODS)
// ============================================================================

/// `organization_periods` table (NHS ODS): operational/legal validity periods
/// for an organization as a whole.
pub mod organization_periods {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `organization_periods` table.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "organization_periods")]
    pub struct Model {
        /// Primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning organization foreign key.
        pub organization_id: Uuid,
        /// Period type code.
        pub period_type: String,
        /// Period start date.
        pub start_date: Option<TimeDate>,
        /// Period end date.
        pub end_date: Option<TimeDate>,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// Belongs to the owning `organizations` row.
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
// Postcode Geography Models
// ============================================================================

/// `postcode_geography` table: reference data mapping UK postcodes to NHS and
/// administrative geographies (LSOA, local authority, ICB, region, etc.).
pub mod postcode_geography {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `postcode_geography` table, keyed by postcode.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "postcode_geography")]
    pub struct Model {
        /// Postcode primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub postcode: String,
        /// 2011 Lower Layer Super Output Area code.
        pub lsoa11: Option<String>,
        /// Local authority code.
        pub local_authority: Option<String>,
        /// Local authority name.
        pub local_authority_name: Option<String>,
        /// Integrated Care Board code.
        pub icb: Option<String>,
        /// Integrated Care Board name.
        pub icb_name: Option<String>,
        /// NHS England region code.
        pub nhs_england_region: Option<String>,
        /// NHS England region name.
        pub nhs_england_region_name: Option<String>,
        /// Parliamentary constituency code.
        pub parliamentary_constituency: Option<String>,
        /// Parliamentary constituency name.
        pub parliamentary_constituency_name: Option<String>,
        /// Government office region.
        pub government_office_region: Option<String>,
        /// Cancer alliance.
        pub cancer_alliance: Option<String>,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// ODS CodeSystem Reference Tables
// ============================================================================

/// `ods_role_references` table: ODS `CodeSystem` lookup mapping role IDs to
/// human-readable role names.
pub mod ods_role_references {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `ods_role_references` table, keyed by role ID.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ods_role_references")]
    pub struct Model {
        /// Role ID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub role_id: String,
        /// Human-readable role name.
        pub role_name: String,
        /// Whether this is a primary role type.
        pub is_primary_role_type: bool,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// `ods_relationship_references` table: ODS `CodeSystem` lookup mapping
/// relationship IDs to human-readable names.
pub mod ods_relationship_references {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `ods_relationship_references` table, keyed by relationship ID.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ods_relationship_references")]
    pub struct Model {
        /// Relationship ID primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub relationship_id: String,
        /// Human-readable relationship name.
        pub relationship_name: String,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// `ods_record_class_references` table: ODS `CodeSystem` lookup mapping
/// record-class codes to names.
pub mod ods_record_class_references {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `ods_record_class_references` table, keyed by code.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ods_record_class_references")]
    pub struct Model {
        /// Record-class code primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub code: String,
        /// Human-readable record-class name.
        pub name: String,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// `ods_record_use_type_references` table: ODS `CodeSystem` lookup mapping
/// record-use-type codes to names.
pub mod ods_record_use_type_references {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `ods_record_use_type_references` table, keyed by code.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "ods_record_use_type_references")]
    pub struct Model {
        /// Record-use-type code primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub code: String,
        /// Human-readable record-use-type name.
        pub name: String,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// `practitioner_role_references` table: lookup mapping practitioner-role
/// codes to names and optional categories.
pub mod practitioner_role_references {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `practitioner_role_references` table, keyed by role code.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "practitioner_role_references")]
    pub struct Model {
        /// Role code primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub role_code: String,
        /// Human-readable role name.
        pub role_name: String,
        /// Optional role category.
        pub role_category: Option<String>,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

/// `geography_name_references` table: lookup mapping ONS geography codes to
/// names and geography types.
pub mod geography_name_references {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A row in the `geography_name_references` table, keyed by ONS code.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "geography_name_references")]
    pub struct Model {
        /// ONS geography code primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub ons_code: String,
        /// Human-readable geography name.
        pub name: String,
        /// Geography type (Local Authority, ICB, …).
        pub geography_type: String,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
    }

    /// Foreign-key relations for this entity (empty when it has none).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Document Models
// ============================================================================

/// The `worker_documents` table: identity documents per worker.
pub mod worker_documents {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// An identity-document row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_documents")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning worker id (FK).
        pub worker_id: Uuid,
        /// Document type (serde tag, e.g. `Passport`).
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

    /// `belongs_to` the parent worker.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `workers`.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }
    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Emergency Contact Models
// ============================================================================

/// The `worker_emergency_contacts` table (address flattened; telecom in
/// `worker_emergency_contact_telecom`).
pub mod worker_emergency_contacts {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// An emergency-contact row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_emergency_contacts")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning worker id (FK).
        pub worker_id: Uuid,
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

    /// `belongs_to` the parent worker.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `workers`.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }
    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

/// The `worker_emergency_contact_telecom` table.
pub mod worker_emergency_contact_telecom {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A telecom row for one emergency contact.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_emergency_contact_telecom")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning emergency-contact id (FK).
        pub emergency_contact_id: Uuid,
        /// Contact system (serde tag).
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
        /// FK to `worker_emergency_contacts`.
        #[sea_orm(
            belongs_to = "super::worker_emergency_contacts::Entity",
            from = "Column::EmergencyContactId",
            to = "super::worker_emergency_contacts::Column::Id"
        )]
        EmergencyContact,
    }
    impl Related<super::worker_emergency_contacts::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::EmergencyContact.def()
        }
    }
    impl ActiveModelBehavior for ActiveModel {}
}

// ============================================================================
// Worker Photo Models
// ============================================================================

/// The `worker_photos` table: photo references per worker.
pub mod worker_photos {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// A photo-reference row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_photos")]
    pub struct Model {
        /// Application-assigned primary key.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// Owning worker id (FK).
        pub worker_id: Uuid,
        /// The photo URL / reference.
        pub url: String,
        /// Ordinal position within the list.
        pub position: i32,
    }

    /// `belongs_to` the parent worker.
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {
        /// FK to `workers`.
        #[sea_orm(
            belongs_to = "super::workers::Entity",
            from = "Column::WorkerId",
            to = "super::workers::Column::Id"
        )]
        Worker,
    }
    impl Related<super::workers::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Worker.def()
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
        /// The entity name (`worker`).
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
/// worker; for v1 the `same_identity` (worker ↔ person) backbone edge
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
        /// This service's originating worker record (its `id`).
        pub from_pid: Uuid,
        /// Edge kind token (§9 registry), e.g. `same_identity`.
        pub kind: String,
        /// The far record's `EntityRef` URN, e.g. `person:<uuid>`.
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
// worker_assessments
// ============================================================================

/// `SeaORM` entity for the `worker_assessments` table — one row per
/// administration of one assessment instrument to one worker
/// (aptitude / personality / psychometric / selection; see
/// [`crate::models::assessment`]). The per-scale outcomes ride in the
/// `results` JSONB array rather than a child table: an assessment is
/// always read and written whole. Persistence helpers (insert, list,
/// find, update, soft-delete) live in [`crate::db::assessments`].
///
/// `category` and `status` store the domain enums' lowercase wire
/// tokens, so the vocabulary can grow by data migration rather than
/// DDL. Deletion is soft (`deleted_at`) so the audit trail stays
/// intact.
pub mod worker_assessments {
    use super::{Deserialize, Serialize};
    use sea_orm::entity::prelude::*;

    /// One recorded assessment row.
    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "worker_assessments")]
    pub struct Model {
        /// Assessment id; application-assigned.
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        /// The assessed worker's `id`.
        pub worker_id: Uuid,
        /// Category wire token: `aptitude` | `personality` |
        /// `psychometric` | `selection`.
        pub category: String,
        /// The instrument's name.
        pub instrument: String,
        /// The test publisher / administering provider.
        pub provider: Option<String>,
        /// Lifecycle wire token: `scheduled` | `in_progress` |
        /// `completed` | `expired` | `cancelled`.
        pub status: String,
        /// The date the assessment was taken.
        pub administered_on: Option<TimeDate>,
        /// The date the results stop counting as current.
        pub expires_on: Option<TimeDate>,
        /// Who administered it.
        pub administered_by: Option<String>,
        /// Operator notes (sensitive — redacted under `mask`).
        pub notes: Option<String>,
        /// The per-scale outcomes, as a JSON array of result objects.
        pub results: Json,
        /// Creation timestamp.
        pub created_at: TimeDateTimeWithTimeZone,
        /// Last-update timestamp.
        pub updated_at: TimeDateTimeWithTimeZone,
        /// Soft-delete timestamp; `None` while live.
        pub deleted_at: Option<TimeDateTimeWithTimeZone>,
        /// SHA-256 over this assessment row
        /// (`crate::compliance::record_integrity`). `None` on rows written
        /// before the column existed; never back-filled, because a
        /// back-fill would certify whatever the current content happens to
        /// be — the claim the hash exists to test.
        pub content_hash: Option<String>,
        /// BLAKE3 digest over the same pre-image as `content_hash`.
        /// `None` on rows written before the second algorithm was
        /// adopted; never back-filled.
        pub content_hash_blake3: Option<String>,
    }

    /// `SeaORM` relations for the assessment entity (none defined — the
    /// worker is referenced by id and joined in the repository layer).
    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
