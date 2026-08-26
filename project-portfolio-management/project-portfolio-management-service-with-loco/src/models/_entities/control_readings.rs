//! `SeaORM` Entity — `control_readings`. **Append-only**: correcting a
//! reading means recording another one, because a control history that
//! can be rewritten measures whatever the editor wanted. See entity
//! spec §5.9.8 / FR-38.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "control_readings")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub control_pid: Uuid,
    pub observed_at: DateTimeWithTimeZone,
    /// `None` is `unmeasured` — a third verdict, never a pass.
    pub value: Option<i64>,
    pub verdict: String,
    pub gap: Option<i64>,
    pub method: String,
    /// An explicit acceptance of a failing reading. A failure with
    /// neither this nor an action is reported as **unanswered**.
    pub accepted_at: Option<DateTimeWithTimeZone>,
    pub accepted_reason: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
