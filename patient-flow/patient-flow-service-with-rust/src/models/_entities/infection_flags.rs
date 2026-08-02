//! `SeaORM` Entity — `infection_flags`. Per-stay IPC precaution flags
//! (spec `infection-control.md`).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "infection_flags")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub stay_pid: Uuid,
    pub precaution: String,
    pub organism: Option<String>,
    pub status: String,
    pub requires_side_room: bool,
    pub flagged_at: DateTimeWithTimeZone,
    pub cleared_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
