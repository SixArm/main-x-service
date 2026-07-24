//! `SeaORM` Entity — `program_placements`. One person's placement on an early-career programme (WPM-R23).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "program_placements")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub program_pid: Uuid,
    pub employee_pid: Uuid,
    pub supervisor_pid: Option<Uuid>,
    pub started_on: Date,
    pub ends_on: Option<Date>,
    pub status: String,
    pub off_the_job_hours: i32,
    pub outcome: String,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
