//! `SeaORM` Entity — `merge_records`. One row per record-merge: which
//! duplicate folded into which survivor, by whom, with a snapshot of the
//! transferred payload.

// SeaORM-generated entity: the field-level shape is documented by the
// migration and the `merge_records` table, not by per-field rustdoc.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "merge_records")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    pub main_pid: Uuid,
    pub duplicate_pid: Uuid,
    pub reason: Option<String>,
    pub actor: Option<String>,
    pub transferred: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
