//! `SeaORM` Entity — `redirects`. `to_path` is null for a `410 Gone`
//! marker: saying a page is gone beats sending a reader somewhere that
//! is not what they asked for (CMS-R10, CMS-R17).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "redirects")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub site_pid: Uuid,
    pub locale: String,
    pub from_path: String,
    pub to_path: Option<String>,
    pub status: i32,
    pub reason: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
