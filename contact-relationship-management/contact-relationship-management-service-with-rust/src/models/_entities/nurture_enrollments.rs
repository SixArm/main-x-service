//! `SeaORM` Entity -- `nurture_enrollments`. One contact through a sequence (CRM-R9); the scheduler is idempotent per (enrolment, step).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "nurture_enrollments")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub sequence_pid: Uuid,
    pub contact_pid: Uuid,
    pub current_step: i32,
    pub next_due_at: Option<DateTimeWithTimeZone>,
    pub status: String,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
