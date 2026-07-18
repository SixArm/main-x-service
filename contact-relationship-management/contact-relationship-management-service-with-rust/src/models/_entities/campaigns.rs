//! `SeaORM` Entity -- `campaigns`. One campaign with simulated engagement counters (CRM-R8).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "campaigns")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub kind: String,
    pub name: String,
    pub status: String,
    pub cost_minor: i64,
    pub currency: String,
    pub segment_pid: Option<Uuid>,
    pub recipients: i32,
    pub delivered: i32,
    pub opened: i32,
    pub clicked: i32,
    pub unsubscribed: i32,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
