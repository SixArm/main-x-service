//! `SeaORM` Entity — `proposals`. Work-intake demand records
//! (PPM-1): the pipeline that stands between an idea and a funded
//! plan.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "proposals")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub kind_target: String,
    pub sponsor_ref: Option<String>,
    pub strategic_rationale: Option<String>,
    pub requested_minor: Option<i64>,
    pub currency: Option<String>,
    pub status: String,
    pub promoted_plan_pid: Option<Uuid>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
