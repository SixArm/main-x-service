//! `SeaORM` Entity — `organizations`. The full
//! `organization_matcher::Organization` payload is stored in `data`
//! (JSONB); `pid` and `name` are denormalised for lookup and listing.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `organizations` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "organizations")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub name: String,
    pub data: Json,
    pub active: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
