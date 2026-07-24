//! `SeaORM` Entity — `assessments`. One administration of one instrument to one candidate or employee (WPM-R20).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "assessments")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub instrument_pid: Uuid,
    pub subject_kind: String,
    pub subject_pid: Uuid,
    pub application_pid: Option<Uuid>,
    pub status: String,
    pub scheduled_on: Option<Date>,
    pub completed_on: Option<Date>,
    pub expires_on: Option<Date>,
    pub administered_by: Option<String>,
    pub notes: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
