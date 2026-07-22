//! `SeaORM` Entity — `scheduled_actions`. The set-and-forget queue: an
//! action configured once, held until `due_at`, then fired by the
//! sweep (endpoint or the env-gated ticker) exactly once.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "scheduled_actions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub subject_kind: String,
    pub subject_pid: Uuid,
    pub action_kind: String,
    pub payload: Json,
    pub due_at: DateTimeWithTimeZone,
    pub status: String,
    pub source_automation_pid: Option<Uuid>,
    pub created_by: Option<String>,
    pub fired_at: Option<DateTimeWithTimeZone>,
    pub outcome: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
