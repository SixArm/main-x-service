//! Migration: **Total Project Control** (TPC) — Devaux's Index of
//! Project Performance (DIPP), Expected Monetary Value (EMV), and Cost
//! Estimate to Complete (CEC). Entity spec §5.9.7 / FR-37.
//!
//! The field set comes from `spec/total-project-control/index.md`.
//!
//! ## Why the ratio is a generated column
//!
//! `dipp_progress_index_ratio` is `GENERATED ALWAYS AS (numerator /
//! denominator) STORED`, exactly as the data dictionary specifies. A
//! ratio written by a handler beside the two numbers that produce it is
//! a value that can disagree with its own inputs after any later edit;
//! a generated column cannot. This is the same instinct as computing
//! every other derived figure on read (entity spec §10.6) — the
//! database is simply the cheapest place to enforce it for a ratio this
//! simple.
//!
//! `NULLIF(denominator, 0)` makes a zero baseline yield `NULL` rather
//! than raising a division error on insert. A zero baseline DIPP is
//! undefined, and `NULL` is how this schema says so; the read path
//! reports the reason (`crate::tpc::Undefined::ZeroDenominator`).
//!
//! ## Money is `NUMERIC`, never a float
//!
//! Every money and ratio column is `NUMERIC`. The service reads them as
//! integer minor units and basis points (`crate::tpc`), so no float
//! touches a currency figure at any layer.

use sea_orm_migration::prelude::*;

/// The Total Project Control migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `total_project_control`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS total_project_control (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     -- ISO 4217. Rankings never cross it: this service
                     -- does not convert currency anywhere.
                     currency VARCHAR NOT NULL,
                     observed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     -- The stored DIPP, which may carry TPC time-value
                     -- terms (acceleration premium, delay cost) that
                     -- EMV alone does not. Divergence from EMV/CEC is
                     -- reported as a finding, never silently resolved.
                     total_project_control_dipp NUMERIC NULL,
                     total_project_control_dipp_progress_index_numerator NUMERIC NULL,
                     total_project_control_dipp_progress_index_denominator NUMERIC NULL,
                     total_project_control_dipp_progress_index_ratio NUMERIC
                         GENERATED ALWAYS AS (
                             total_project_control_dipp_progress_index_numerator
                             / NULLIF(total_project_control_dipp_progress_index_denominator, 0)
                         ) STORED,
                     -- May be negative: a project can be worth less
                     -- than nothing to finish, and that is the case the
                     -- metric exists to expose. Deliberately unchecked.
                     total_project_control_expected_monetary_value NUMERIC NOT NULL,
                     -- No cost estimate to complete is negative.
                     total_project_control_cost_estimate_to_complete NUMERIC NOT NULL
                         CHECK (total_project_control_cost_estimate_to_complete >= 0),
                     deleted_at TIMESTAMPTZ NULL
                 );
                 -- The per-plan read: latest observation first.
                 CREATE INDEX IF NOT EXISTS total_project_control_plan
                     ON total_project_control (plan_pid, observed_at DESC);
                 -- The portfolio triage read: rank by DIPP within one
                 -- currency (entity spec FR-37).
                 CREATE INDEX IF NOT EXISTS total_project_control_currency
                     ON total_project_control (currency, observed_at DESC);",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS total_project_control;")
            .await?;
        Ok(())
    }
}
