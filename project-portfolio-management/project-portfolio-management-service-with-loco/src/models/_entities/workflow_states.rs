//! `SeaORM` Entity — `workflow_states`. See entity spec §5.9.1.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "workflow_states")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub workflow_pid: Uuid,
    pub state_key: String,
    pub label: String,
    /// `todo` | `active` | `waiting` | `done`. **`NOT NULL`** — every
    /// derived view computes from this rather than from the name.
    pub category: String,
    pub wip_limit: Option<i32>,
    pub is_initial: bool,
    pub is_terminal: bool,
    pub position: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
