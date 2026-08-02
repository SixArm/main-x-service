//! `SeaORM` Entity — `budget_lines`. Planned vs actual spend per work
//! item (PPM-10). Money is integer **minor units** + ISO-4217 code —
//! exact arithmetic, no floats.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "budget_lines")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    pub category: String,
    pub description: String,
    pub currency: String,
    pub planned_minor: i64,
    pub actual_minor: i64,
    pub period_start: Option<Date>,
    pub period_end: Option<Date>,
    pub gate: Option<String>,
    pub released_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
