//! `SeaORM` Entity — `transfers`. Immutable record of each move
//! (admission placement, ward/bed transfer, discharge).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "transfers")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub stay_pid: Uuid,
    pub from_bed_pid: Option<Uuid>,
    pub to_bed_pid: Option<Uuid>,
    pub reason: String,
    pub moved_at: DateTimeWithTimeZone,
    pub moved_by_ref: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
