//! `SeaORM` Entity — `requisitions`. One funded job opening with its hiring pipeline (HCM-R1).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "requisitions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub organization_ref: String,
    pub department: String,
    pub job_title: String,
    pub headcount: i32,
    pub salary_min_minor: Option<i64>,
    pub salary_max_minor: Option<i64>,
    pub salary_currency: Option<String>,
    pub status: String,
    pub opened_on: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
