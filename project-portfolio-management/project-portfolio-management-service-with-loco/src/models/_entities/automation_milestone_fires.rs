//! `SeaORM` Entity — `automation_milestone_fires`. The exactly-once
//! claim for the `milestone_due` trigger (FR-32): one row per
//! `(automation_pid, milestone_pid)` that has fired. Inserting is the
//! claim — `ON CONFLICT DO NOTHING`, so only the caller that actually
//! adds the row applies the rule's actions.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "automation_milestone_fires")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    pub automation_pid: Uuid,
    pub milestone_pid: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
