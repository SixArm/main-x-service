//! `SeaORM` Entity — `employees`. One employment relationship: the single source of employment truth (WPM-R7). Identities are `EntityRef` URNs; salary is minor units (sensitive).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "employees")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub person_ref: String,
    pub worker_ref: Option<String>,
    pub organization_ref: String,
    pub employee_number: String,
    pub display_name: String,
    pub status: String,
    pub employment_type: String,
    pub fte_percent: i32,
    pub department: String,
    pub job_title: String,
    pub manager_pid: Option<Uuid>,
    pub salary_minor: Option<i64>,
    pub salary_currency: Option<String>,
    pub hired_on: Date,
    pub terminated_on: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
