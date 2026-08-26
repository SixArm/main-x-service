//! `SeaORM` Entity — `total_project_control`. Devaux's Total Project
//! Control observations per plan. See entity spec §5.9.7 / FR-37 and
//! `spec/total-project-control/index.md`.

// SeaORM-generated entity shape: documented by the migration.
#![allow(missing_docs)]

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "total_project_control")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub pid: Uuid,
    pub plan_pid: Uuid,
    /// ISO 4217. Rankings never cross it — this service does not
    /// convert currency anywhere.
    pub currency: String,
    pub observed_at: DateTimeWithTimeZone,
    /// The stored DIPP in basis points, which may carry TPC time-value
    /// terms that EMV alone does not. Divergence from EMV/CEC is
    /// reported as a finding, never silently resolved.
    pub total_project_control_dipp: Option<Decimal>,
    pub total_project_control_dipp_progress_index_numerator: Option<Decimal>,
    pub total_project_control_dipp_progress_index_denominator: Option<Decimal>,
    /// `GENERATED ALWAYS` in Postgres, so it cannot disagree with the
    /// two numbers beside it. Never written by this crate.
    pub total_project_control_dipp_progress_index_ratio: Option<Decimal>,
    /// Minor units. **May be negative**: a project can be worth less
    /// than nothing to finish.
    pub total_project_control_expected_monetary_value: Decimal,
    /// Minor units. Never negative (CHECK constraint).
    pub total_project_control_cost_estimate_to_complete: Decimal,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
