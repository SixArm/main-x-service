//! `SeaORM` Entity — `automations`. One workflow rule: when a trigger
//! fires (typically a task crossing the Kanban board), apply one
//! action. `plan_pid` scopes the rule to a plan; `NULL` = every plan.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "automations")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Option<Uuid>,
    pub name: String,
    pub trigger_kind: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub action_kind: String,
    pub action_value: Json,
    pub enabled: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
