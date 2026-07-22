//! `SeaORM` Entity — `automation_runs`. The append-only log of what an
//! automation actually did (or refused to do) on one subject: the
//! honest record behind "the platform updated this for you".

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "automation_runs")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub automation_pid: Uuid,
    pub subject_kind: String,
    pub subject_pid: Uuid,
    pub outcome: String,
    pub detail: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
