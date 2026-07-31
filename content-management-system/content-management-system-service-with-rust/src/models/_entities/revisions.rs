//! `SeaORM` Entity — `revisions`. **Append-only**: rows are never
//! updated and never deleted, and there is no `deleted_at` to soft-
//! delete one with (CMS-D3). A restore writes a new row carrying
//! `restored_from_pid`; an erasure blanks `blocks`/`fields` and keeps
//! the row, its number, and its linkage (spec `audit.md`).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "revisions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub variant_pid: Uuid,
    pub number: i32,
    pub title: String,
    pub blocks: Json,
    pub fields: Json,
    pub seo: Json,
    pub type_schema_version: i32,
    pub author_ref: Option<String>,
    pub note: Option<String>,
    pub restored_from_pid: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
