//! `SeaORM` Entity — `tasks`. The per-work-item operational task
//! sub-resource (spec §13): Kanban status + honest flow stamps
//! (`status_changed_at`, first `done_at`).

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub work_item_pid: Uuid,
    pub sprint_pid: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub assignee_ref: Option<String>,
    pub status_changed_at: DateTimeWithTimeZone,
    pub done_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
