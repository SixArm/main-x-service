//! `SeaORM` Entity — `benefits`. Value-realization records (PPM-11):
//! financial targets in minor units, non-financial as notes.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "benefits")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    pub title: String,
    pub category: String,
    pub currency: Option<String>,
    pub target_minor: Option<i64>,
    pub realized_minor: i64,
    pub target_note: Option<String>,
    pub realized_note: Option<String>,
    pub expected_on: Option<Date>,
    pub status: String,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
