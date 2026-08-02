//! `SeaORM` Entity — `bed_requests`. The demand queue the allocator
//! serves (spec `domain-model.md`).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bed_requests")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub person_ref: String,
    pub origin: String,
    pub target_ward_pid: Option<Uuid>,
    pub specialty: Option<String>,
    pub priority: String,
    pub requirements: Json,
    pub status: String,
    pub allocated_bed_pid: Option<Uuid>,
    pub requested_at: DateTimeWithTimeZone,
    pub resolved_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
