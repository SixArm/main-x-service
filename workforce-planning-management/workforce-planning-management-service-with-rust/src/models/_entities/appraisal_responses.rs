//! `SeaORM` Entity — `appraisal_responses`. One rater's 360° response (WPM-R29): per-competency scores + optional comment. Links to its nomination by design (WPM-D21 — procedural anonymity: the link enforces once-per-rater; the API never serves rater-level content).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "appraisal_responses")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub appraisal_pid: Uuid,
    #[sea_orm(unique)]
    pub nomination_pid: Uuid,
    pub rater_group: String,
    pub scores: Json,
    pub comment: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
