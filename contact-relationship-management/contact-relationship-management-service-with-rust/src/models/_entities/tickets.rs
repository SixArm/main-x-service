//! `SeaORM` Entity -- `tickets`. One support ticket with derived SLA deadlines + breach facts (CRM-R10/R11). NOT a `case`-registry identity.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tickets")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub contact_pid: Option<Uuid>,
    pub account_pid: Option<Uuid>,
    pub assignee_ref: Option<String>,
    pub title: String,
    pub priority: String,
    pub channel: String,
    pub status: String,
    pub opened_at: DateTimeWithTimeZone,
    pub first_response_due_at: Option<DateTimeWithTimeZone>,
    pub resolution_due_at: Option<DateTimeWithTimeZone>,
    pub first_responded_at: Option<DateTimeWithTimeZone>,
    pub resolved_at: Option<DateTimeWithTimeZone>,
    pub first_response_breached: bool,
    pub resolution_breached: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
