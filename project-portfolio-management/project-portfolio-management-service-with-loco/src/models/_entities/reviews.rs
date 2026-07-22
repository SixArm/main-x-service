//! `SeaORM` Entity — `reviews`. A collaborative-review invitation: one
//! subject (idea / proposal / plan) delegated to one internal or
//! external expert, with their verdict once submitted.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "reviews")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub subject_kind: String,
    pub subject_pid: Uuid,
    pub reviewer_ref: String,
    pub reviewer_scope: String,
    pub expertise: Option<String>,
    pub status: String,
    pub due_on: Option<Date>,
    pub score: Option<i32>,
    pub recommendation: Option<String>,
    pub comment: Option<String>,
    pub invited_by: Option<String>,
    pub responded_at: Option<DateTimeWithTimeZone>,
    pub submitted_at: Option<DateTimeWithTimeZone>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
