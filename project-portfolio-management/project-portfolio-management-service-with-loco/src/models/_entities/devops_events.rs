//! `SeaORM` Entity — `devops_events`. Ingested deploy / incident /
//! recovery events: the only source the DORA-style metrics derive
//! from (nothing is inferred from data that was never ingested).

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "devops_events")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub work_item_pid: Uuid,
    pub kind: String,
    pub environment: Option<String>,
    pub version: Option<String>,
    pub reference: Option<String>,
    pub incident_pid: Option<Uuid>,
    pub caused_by_deploy_pid: Option<Uuid>,
    pub occurred_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
