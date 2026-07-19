//! `SeaORM` Entity — `risks`. Portfolio-level risk records (PPM-12)
//! alongside a work item; exposure = probability × impact (derived).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "risks")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub work_item_pid: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub probability: i32,
    pub impact: i32,
    pub status: String,
    pub owner_ref: Option<String>,
    pub mitigation: Option<String>,
    pub review_date: Option<Date>,
    pub escalated_at: Option<DateTimeWithTimeZone>,
    pub category: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
