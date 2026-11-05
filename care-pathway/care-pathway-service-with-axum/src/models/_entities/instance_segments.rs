//! `SeaORM` Entity — `instance_segments`. A recorded interval on a
//! pathway instance, classified by whether it added value. The
//! primitive time-based analysis is built on — see
//! `spec/time-based-analysis.md` §5.1.

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "instance_segments")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub instance_pid: Uuid,
    pub label: String,
    pub stage: String,
    pub category: String,
    pub waste: Option<String>,
    pub started_at: DateTimeWithTimeZone,
    /// `None` while the segment is still running.
    pub ended_at: Option<DateTimeWithTimeZone>,
    pub actor_ref: Option<String>,
    pub location_ref: Option<String>,
    pub note: Option<String>,
    pub position: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
