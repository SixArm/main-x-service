//! `SeaORM` Entity — `content_references`. The edge index extracted
//! from every saved revision (CMS-D8): what this revision points at, so
//! "where used" is a lookup and a delete that would break something can
//! be refused rather than discovered by a reader.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "content_references")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub from_revision_pid: Uuid,
    pub from_variant_pid: Uuid,
    pub kind: String,
    pub to_entry_pid: Option<Uuid>,
    pub to_asset_pid: Option<Uuid>,
    pub to_entity_ref: Option<String>,
    pub field_key: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
