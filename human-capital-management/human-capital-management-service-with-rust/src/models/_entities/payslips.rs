//! `SeaORM` Entity — `payslips`. One employee's payslip in one run: gross, deduction lines (JSONB), net (HCM-R13).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "payslips")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub run_pid: Uuid,
    pub employee_pid: Uuid,
    pub currency: String,
    pub gross_minor: i64,
    pub deductions: Json,
    pub net_minor: i64,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
