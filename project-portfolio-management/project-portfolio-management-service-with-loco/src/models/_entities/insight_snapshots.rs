//! `SeaORM` Entity — `insight_snapshots`. One row per point-in-time
//! estate capture (kind + JSONB body) behind the board/CRO trends.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "insight_snapshots")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub taken_at: DateTimeWithTimeZone,
    pub kind: String,
    pub body: Json,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
