//! `SeaORM` Entity -- `deals`. One revenue opportunity moving through stages; terminal stages close it (CRM-R4).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "deals")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub account_pid: Option<Uuid>,
    pub primary_contact_pid: Option<Uuid>,
    pub owner_ref: Option<String>,
    pub pipeline_pid: Uuid,
    pub stage_pid: Uuid,
    pub name: String,
    pub amount_minor: i64,
    pub currency: String,
    pub expected_close_on: Option<Date>,
    pub kanban_position: i32,
    pub source_campaign_pid: Option<Uuid>,
    pub closed_at: Option<DateTimeWithTimeZone>,
    pub won: bool,
    pub lost_reason: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
