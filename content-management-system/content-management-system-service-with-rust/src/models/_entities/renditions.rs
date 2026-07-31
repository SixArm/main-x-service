//! `SeaORM` Entity — `renditions`. A **declared** derived variant of an
//! asset: dimensions, format, and a state. `storage_ref` stays null
//! until something produces the bytes, and delivery reports only what
//! exists — a declared-but-unproduced rendition is never served as a
//! URL that 404s (spec `assets.md`).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "renditions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub asset_pid: Uuid,
    pub key: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub format: String,
    pub storage_ref: Option<String>,
    pub state: String,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
