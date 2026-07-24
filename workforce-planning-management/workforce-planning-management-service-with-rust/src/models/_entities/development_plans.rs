//! `SeaORM` Entity — `development_plans`. One employee's upskilling or reskilling plan (WPM-R21).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "development_plans")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub employee_pid: Uuid,
    pub kind: String,
    pub target_job_title: Option<String>,
    pub target_department: Option<String>,
    pub rationale: Option<String>,
    pub status: String,
    pub started_on: Option<Date>,
    pub target_on: Option<Date>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
