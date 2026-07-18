//! `SeaORM` Entity — `work_items`. The full `project_portfolio_management_matcher::WorkItem`
//! payload is stored in `data` (JSONB); `pid`, `kind`, `name`, and
//! `portfolio_pid` are denormalised for scoping, lookup, and listing.
//!
//! The four REST collections (portfolio / project / product / program)
//! share this one table; the `kind` column is the collection, and every
//! query is scoped by it so matching never crosses collections.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted work-item row.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "work_items")]
pub struct Model {
    /// Row creation timestamp.
    pub created_at: DateTimeWithTimeZone,
    /// Row last-update timestamp.
    pub updated_at: DateTimeWithTimeZone,
    /// Internal auto-increment primary key.
    #[sea_orm(primary_key)]
    pub id: i32,
    /// Public, externally exposed identifier (UUID v4).
    #[sea_orm(unique)]
    pub pid: Uuid,
    /// The collection / kind this row belongs to (`Portfolio` / `Project`
    /// / `Product` / `Program`), denormalised so queries scope to one
    /// collection without parsing the JSON payload.
    pub kind: String,
    /// Denormalised work-item name, for lookup and listing.
    pub name: String,
    /// The full `project_portfolio_management_matcher::WorkItem` payload (JSONB).
    pub data: Json,
    /// Denormalised parent portfolio pid for child kinds (null for the
    /// `Portfolio` kind), for cheap roll-up of a portfolio's children.
    pub portfolio_pid: Option<Uuid>,
    /// Whether the row is active (cleared on soft-delete).
    pub active: bool,
    /// Operational phase-gate stage: the highest gate passed
    /// (`g0_concept` … `g5_benefits`), `None` before the first
    /// approved gate review. Governance state, not matcher payload.
    pub stage: Option<String>,
    /// Soft-delete timestamp; `None` while active.
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
