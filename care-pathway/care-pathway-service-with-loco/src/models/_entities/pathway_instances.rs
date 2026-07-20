//! `SeaORM` Entity — `pathway_instances` (care-pathway instance layer).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "pathway_instances")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub pathway_pid: Uuid,
    pub subject_ref: String,
    pub status: String,
    pub urgency: String,
    pub enrolled_on: Date,
    pub next_review_on: Option<Date>,
    pub closed_on: Option<Date>,
    pub closure_reason: Option<String>,
    pub outcome: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
