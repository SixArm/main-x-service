//! `SeaORM` Entity -- `activities`. One interaction, attached to any relationship object (CRM-R2).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "activities")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub subject_kind: String,
    pub subject_pid: Uuid,
    pub kind: String,
    pub occurred_at: DateTimeWithTimeZone,
    pub actor_ref: Option<String>,
    pub summary: String,
    pub due_on: Option<Date>,
    pub done: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
