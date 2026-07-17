//! `SeaORM` Entity — `stays`. One inpatient episode for one patient
//! (`person_ref` URN), admission → transfers → discharge, with the
//! SAFER fields and the DTOC anchor (spec `domain-model.md`).

#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "stays")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub person_ref: String,
    pub display_name: String,
    pub status: String,
    pub admitted_at: DateTimeWithTimeZone,
    pub source: String,
    pub ward_pid: Option<Uuid>,
    pub bed_pid: Option<Uuid>,
    pub home_location_note: Option<String>,
    pub named_nurse_ref: Option<String>,
    pub consultant_ref: Option<String>,
    pub senior_review_at: Option<DateTimeWithTimeZone>,
    pub edd: Option<Date>,
    pub ccd: Option<String>,
    pub ccd_met: bool,
    pub discharge_pathway: Option<String>,
    pub discharge_ready_at: Option<DateTimeWithTimeZone>,
    pub discharged_at: Option<DateTimeWithTimeZone>,
    pub discharge_destination: Option<String>,
    pub alerts: Json,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
