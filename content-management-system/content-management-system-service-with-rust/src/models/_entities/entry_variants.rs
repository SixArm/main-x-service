//! `SeaORM` Entity — `entry_variants`. One entry in one locale: its own
//! status, revisions, publish pointer, and schedule, so French can be in
//! review while English is live (CMS-R13).
//!
//! `current_revision_pid` is what was last saved; `published_revision_pid`
//! is what delivery serves. They are different columns because "saved"
//! and "live" are different facts (CMS-D3).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "entry_variants")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub entry_pid: Uuid,
    pub locale: String,
    pub status: String,
    pub current_revision_pid: Option<Uuid>,
    pub published_revision_pid: Option<Uuid>,
    pub translation_of_revision_pid: Option<Uuid>,
    pub reviewer_ref: Option<String>,
    pub scheduled_publish_at: Option<DateTimeWithTimeZone>,
    pub scheduled_unpublish_at: Option<DateTimeWithTimeZone>,
    pub locked_by_ref: Option<String>,
    pub locked_until: Option<DateTimeWithTimeZone>,
    pub published_at: Option<DateTimeWithTimeZone>,
    pub first_published_at: Option<DateTimeWithTimeZone>,
    pub translation_status: Option<String>,
    pub translation_requested_at: Option<DateTimeWithTimeZone>,
    pub translation_requested_by: Option<String>,
    pub translation_due_on: Option<Date>,
    pub translator_ref: Option<String>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
