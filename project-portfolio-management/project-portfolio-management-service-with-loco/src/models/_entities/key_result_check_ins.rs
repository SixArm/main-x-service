//! `SeaORM` Entity — `key_result_check_ins`. **Append-only**: correcting
//! a check-in means recording another. See entity spec §5.9.2.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "key_result_check_ins")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub key_result_pid: Uuid,
    pub observed_at: DateTimeWithTimeZone,
    pub value: i64,
    /// Recorded, and **never blended into the score**.
    pub confidence: Option<i16>,
    pub note: Option<String>,
    pub actor: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
