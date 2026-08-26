//! `SeaORM` Entity — `workflows`. A configurable task or issue state
//! vocabulary. See entity spec §5.9.1 / FR-26.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "workflows")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    /// `None` scopes the workflow deployment-wide; a value scopes it to
    /// that plan, which then overrides the default.
    pub plan_pid: Option<Uuid>,
    pub name: String,
    /// `task` | `issue`.
    pub applies_to: String,
    pub is_default: bool,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
