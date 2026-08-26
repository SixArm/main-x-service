//! Migration: **recorded effort** and the **capacity** it is measured
//! against (entity spec §5.9.3 / §5.9.6, FR-28 / FR-35).
//!
//! ## Non-working time is a table, not a flag
//!
//! `non_working_periods` exists so that leave, study leave, holiday and
//! non-project duty **subtract from the denominator** rather than
//! counting as idle capacity. Without it, somebody on leave for a
//! fortnight reports 0% utilisation, which reads as measured idleness
//! and is the single most defamatory number this service could publish.
//!
//! ## Effort is an assertion
//!
//! A row here is typed by a person, unlike a task transition, which is
//! a by-product of the work. Every roll-up over this table is labelled
//! `asserted` for that reason (`crate::effort`).
//!
//! ## What this deliberately does not enable
//!
//! Per-person **utilisation** is permitted by the 2026-08-25 decision
//! (`agents/share/time-based-analysis.md` §7.1). Per-person **cycle
//! time, throughput and flow efficiency** remain refused, and nothing
//! in this schema or its read paths computes them.

use sea_orm_migration::prelude::*;

/// The effort migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the effort and capacity tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS time_entries (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     task_pid UUID NULL,
                     -- `person:` / `worker:` EntityRef URN.
                     actor_ref VARCHAR NOT NULL,
                     spent_on DATE NOT NULL,
                     minutes BIGINT NOT NULL CHECK (minutes > 0),
                     category VARCHAR NOT NULL DEFAULT 'unclassified'
                         CHECK (category IN ('capex', 'opex', 'unclassified')),
                     billable BOOLEAN NOT NULL DEFAULT false,
                     note VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS time_entries_plan
                     ON time_entries (plan_pid, spent_on);
                 CREATE INDEX IF NOT EXISTS time_entries_actor
                     ON time_entries (actor_ref, spent_on);
                 CREATE INDEX IF NOT EXISTS time_entries_task ON time_entries (task_pid);

                 CREATE TABLE IF NOT EXISTS working_time_configs (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     -- NULL is the deployment-wide default.
                     scope_ref VARCHAR NULL,
                     minutes_per_day INTEGER NOT NULL CHECK (minutes_per_day > 0),
                     working_days_per_week INTEGER NOT NULL
                         CHECK (working_days_per_week BETWEEN 1 AND 7),
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS working_time_configs_scope
                     ON working_time_configs (scope_ref) WHERE deleted_at IS NULL;

                 CREATE TABLE IF NOT EXISTS non_working_periods (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     person_ref VARCHAR NOT NULL,
                     starts_on DATE NOT NULL,
                     ends_on DATE NOT NULL,
                     kind VARCHAR NOT NULL
                         CHECK (kind IN ('leave', 'holiday', 'study_leave', 'non_project_duty')),
                     note VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL,
                     CONSTRAINT non_working_periods_ordered CHECK (ends_on >= starts_on)
                 );
                 CREATE INDEX IF NOT EXISTS non_working_periods_person
                     ON non_working_periods (person_ref, starts_on);",
            )
            .await?;
        Ok(())
    }

    /// Drop the three tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS non_working_periods;
                 DROP TABLE IF EXISTS working_time_configs;
                 DROP TABLE IF EXISTS time_entries;",
            )
            .await?;
        Ok(())
    }
}
