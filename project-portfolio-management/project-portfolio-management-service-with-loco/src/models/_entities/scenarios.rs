//! `SeaORM` Entity — `scenarios`. What-if candidate portfolios
//! (PPM-4): a membership set + constraint knobs, evaluated on read.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "scenarios")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub members: Json,
    pub budget_cap_minor: Option<i64>,
    pub currency: Option<String>,
    pub must_include: Json,
    pub status: String,
    pub committed_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
