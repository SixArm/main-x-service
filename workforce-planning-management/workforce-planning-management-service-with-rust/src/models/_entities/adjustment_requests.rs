//! `SeaORM` Entity — `adjustment_requests`. One reasonable-adjustment request (WPM-R33): barrier, impact, change — **no diagnosis column exists** (WPM-D25).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "adjustment_requests")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub employee_pid: Uuid,
    pub category: String,
    pub barrier: String,
    pub impact: String,
    pub adjustment: String,
    pub status: String,
    pub decision_note: Option<String>,
    pub decided_on: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
