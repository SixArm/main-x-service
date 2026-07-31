//! `SeaORM` Entity — `webhook_deliveries`. An attempt log: a row exists
//! because a delivery was tried, which is what makes "why did our CDN
//! not purge?" answerable.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_deliveries")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub webhook_pid: Uuid,
    pub event_id: Uuid,
    pub event_kind: String,
    pub attempt: i32,
    pub state: String,
    pub status_code: Option<i32>,
    pub error: Option<String>,
    pub delivered_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
