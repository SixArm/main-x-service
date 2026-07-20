//! `SeaORM` Entity -- `contacts`. One relationship wrapper over a `person:` record (CRM-R1); `marketing_consent` is send-path law (CRM-D6).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "contacts")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub person_ref: String,
    pub account_pid: Option<Uuid>,
    pub owner_ref: Option<String>,
    pub display_name: String,
    pub status: String,
    pub job_title: Option<String>,
    pub preferred_channel: String,
    pub marketing_consent: String,
    pub consent_changed_at: Option<DateTimeWithTimeZone>,
    pub stakeholder_role: Option<String>,
    pub influence: Option<i32>,
    pub interest: Option<i32>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
