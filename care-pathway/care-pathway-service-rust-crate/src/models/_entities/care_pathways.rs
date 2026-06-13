//! `SeaORM` Entity — `care_pathways`. The full
//! `care_pathway_matcher::CarePathway` payload is stored in `data`
//! (JSONB); `pid` and `name` are denormalised for lookup and listing.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// A persisted care-pathway row.
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "care_pathways")]
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
    /// Denormalised pathway name, for lookup and listing.
    pub name: String,
    /// The full `care_pathway_matcher::CarePathway` payload (JSONB).
    pub data: Json,
    /// Whether the row is active (cleared on soft-delete).
    pub active: bool,
    /// Soft-delete timestamp; `None` while active.
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

/// `SeaORM` relations for [`Entity`] (none defined).
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
