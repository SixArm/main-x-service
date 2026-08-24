//! `SeaORM` Entity — `task_transitions`. The append-only task status
//! transition log, from which every time-based-analysis interval is
//! derived. See `spec/time-based-analysis.md` §5.1.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "task_transitions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub task_pid: Uuid,
    /// Denormalised so a whole board is one query.
    pub plan_pid: Uuid,
    /// `None` marks the task's creation.
    pub from_status: Option<String>,
    pub to_status: String,
    pub at: DateTimeWithTimeZone,
    pub actor_ref: Option<String>,
    pub assignee_ref: Option<String>,
    /// Synthesised by the migration rather than observed — reported by
    /// every analysis so thin evidence stays visible.
    pub backfilled: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
