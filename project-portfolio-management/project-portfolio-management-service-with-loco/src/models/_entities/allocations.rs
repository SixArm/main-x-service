//! `SeaORM` Entity — `allocations`. A person's percentage allocation
//! to a plan over a timeframe (PPM-8). The person is an
//! `EntityRef` URN — never copied demographics, never a matcher signal.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "allocations")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    pub person_ref: String,
    pub role: Option<String>,
    pub percent: i32,
    pub start_date: Option<Date>,
    pub end_date: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
