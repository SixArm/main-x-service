//! `SeaORM` Entity — `routes`. The published path of a routable
//! variant. Superseded rows are kept with `is_current = false`, so a
//! page's address history is answerable (CMS-R17).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "routes")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub site_pid: Uuid,
    pub locale: String,
    pub path: String,
    pub variant_pid: Uuid,
    pub is_current: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
