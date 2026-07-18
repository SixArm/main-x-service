//! `SeaORM` Entity -- `articles`. One knowledge-base article; published edits bump `version` (CRM-R12).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "articles")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub title: String,
    pub body: String,
    pub keywords: Option<String>,
    pub status: String,
    pub version: i32,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
