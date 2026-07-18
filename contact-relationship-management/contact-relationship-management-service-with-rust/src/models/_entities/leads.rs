//! `SeaORM` Entity -- `leads`. One unqualified prospect with its derived score (CRM-R3).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "leads")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub source: String,
    pub campaign_pid: Option<Uuid>,
    pub contact_pid: Option<Uuid>,
    pub display_name: String,
    pub email: Option<String>,
    pub email_domain: Option<String>,
    pub score: i32,
    pub campaign_click: bool,
    pub unsubscribed: bool,
    pub status: String,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
