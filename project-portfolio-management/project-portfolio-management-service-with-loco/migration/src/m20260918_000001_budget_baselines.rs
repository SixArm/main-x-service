//! Migration: the **phased budget baseline** (T-28b, spec §13).
//!
//! ## Append-only, versioned
//!
//! No update or delete path, matching `phase_transitions` and
//! `task_transitions`: a baseline that could be rewritten after the
//! fact would let a forecast be flattered retroactively. A re-baseline
//! is a **new row** at `version = previous + 1`; the old version's
//! periods stay exactly as they were, so any earlier forecast that
//! named its baseline version stays reproducible.
//!
//! ## One currency per baseline
//!
//! A baseline is planned cost per period, in one currency — the same
//! posture `budget_lines`/`business_case_targets` already take on
//! money. A plan whose spend genuinely spans currencies gets one
//! baseline per currency (each its own `plan_pid` + `currency` +
//! `version` sequence), never one baseline mixing them.
//!
//! ## A re-baseline needs a reason; the first one does not
//!
//! `version = 1` is an initial approval — nothing preceded it to
//! explain a change from. `version > 1` is enforced at the handler to
//! carry a `reason`, the same distinction `phase_transitions` draws
//! between a plan's first phase (`from_phase IS NULL`) and a later
//! regression (needs a `reason`).

use sea_orm_migration::prelude::*;

/// The budget-baseline migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `budget_baselines` and `budget_baseline_periods`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS budget_baselines (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     version INTEGER NOT NULL CHECK (version > 0),
                     currency VARCHAR NOT NULL,
                     approved_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     -- Required by the handler for version > 1: a
                     -- re-baseline with no reason is indistinguishable
                     -- from a mistake.
                     reason VARCHAR NULL
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS budget_baselines_plan_version
                     ON budget_baselines (plan_pid, version);
                 CREATE INDEX IF NOT EXISTS budget_baselines_plan
                     ON budget_baselines (plan_pid, version DESC);

                 CREATE TABLE IF NOT EXISTS budget_baseline_periods (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     baseline_pid UUID NOT NULL,
                     period_start DATE NOT NULL,
                     period_end DATE NOT NULL CHECK (period_end >= period_start),
                     planned_minor BIGINT NOT NULL CHECK (planned_minor >= 0)
                 );
                 CREATE INDEX IF NOT EXISTS budget_baseline_periods_baseline
                     ON budget_baseline_periods (baseline_pid, period_start);",
            )
            .await?;
        Ok(())
    }

    /// Drop both tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS budget_baseline_periods;
                 DROP TABLE IF EXISTS budget_baselines;",
            )
            .await?;
        Ok(())
    }
}
