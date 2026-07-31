//! `SeaORM` Entity — `entries`. One piece of content at identity level;
//! its per-locale rows are `entry_variants`, which are the unit of
//! workflow (CMS-R3, CMS-R13).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "entries")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub site_pid: Uuid,
    pub content_type_key: String,
    pub type_schema_version: i32,
    pub key: String,
    pub source_locale: String,
    pub owner_ref: Option<String>,
    pub archived_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
