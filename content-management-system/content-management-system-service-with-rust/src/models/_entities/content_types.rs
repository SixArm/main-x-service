//! `SeaORM` Entity — `content_types`. The operator-declared field
//! schema for a kind of content; `fields` is JSONB because its shape is
//! declared at runtime (CMS-D2), and `schema_version` records which
//! declaration a stored revision was written under (CMS-R2).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "content_types")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub site_pid: Uuid,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub fields: Json,
    pub routable: bool,
    pub template_key: Option<String>,
    pub schema_version: i32,
    pub unpublish_on_stale: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
