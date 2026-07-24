//! `SeaORM` Entity — `assessment_results`. One scale's outcome within an assessment; scores are integers (WPM-R20).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "assessment_results")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub assessment_pid: Uuid,
    pub scale: String,
    pub raw_score: Option<i32>,
    pub max_score: Option<i32>,
    pub percentile: Option<i32>,
    pub band: Option<String>,
    pub narrative: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
