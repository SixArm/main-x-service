//! `SeaORM` Entity — `control_actions`. The fourth step of the
//! Controlling process: what a failing reading provoked. Converts into
//! the task and issue stores that already exist rather than becoming a
//! fifth one. See entity spec §5.9.8 / FR-38.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "control_actions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub reading_pid: Uuid,
    pub kind: String,
    pub description: String,
    pub owner_ref: Option<String>,
    pub due_date: Option<Date>,
    pub converted_task_pid: Option<Uuid>,
    pub converted_issue_pid: Option<Uuid>,
    pub closed_at: Option<DateTimeWithTimeZone>,
    pub outcome: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
