//! Migration: the talent-strategy tables — **development plans**
//! (upskilling and reskilling) with their per-skill items, **talent
//! pipelines** with their members, and **early-career programmes**
//! (apprenticeships / internships / graduate schemes) with their
//! placements. Plus two columns on `succession_plans` that succession
//! planning needs to be more than a list: the incumbent's risk of loss
//! and the vacancy horizon.
//!
//! All declared / recorded data; every rollup (bench strength, plan
//! progress, pipeline health, conversion rate) is **derived** in the
//! controller from these rows, never stored.

use sea_orm_migration::prelude::*;

/// The talent-strategy migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the six tables and extend `succession_plans`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE succession_plans
                     ADD COLUMN IF NOT EXISTS risk_of_loss VARCHAR NULL,
                     ADD COLUMN IF NOT EXISTS vacancy_expected_on DATE NULL;
                 CREATE TABLE IF NOT EXISTS development_plans (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     employee_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL,
                     target_job_title VARCHAR NULL,
                     target_department VARCHAR NULL,
                     rationale VARCHAR NULL,
                     status VARCHAR NOT NULL DEFAULT 'draft',
                     started_on DATE NULL,
                     target_on DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS development_plans_employee
                     ON development_plans (employee_pid);
                 CREATE INDEX IF NOT EXISTS development_plans_kind
                     ON development_plans (kind);
                 CREATE TABLE IF NOT EXISTS development_plan_items (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     skill_pid UUID NOT NULL,
                     current_level INTEGER NOT NULL,
                     target_level INTEGER NOT NULL,
                     method VARCHAR NOT NULL,
                     course_ref VARCHAR NULL,
                     due_on DATE NULL,
                     status VARCHAR NOT NULL DEFAULT 'planned',
                     UNIQUE (plan_pid, skill_pid)
                 );
                 CREATE INDEX IF NOT EXISTS development_plan_items_plan
                     ON development_plan_items (plan_pid);
                 CREATE TABLE IF NOT EXISTS talent_pipelines (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     name VARCHAR NOT NULL UNIQUE,
                     purpose VARCHAR NOT NULL,
                     target_job_title VARCHAR NULL,
                     target_department VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE TABLE IF NOT EXISTS pipeline_members (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     pipeline_pid UUID NOT NULL,
                     subject_kind VARCHAR NOT NULL,
                     subject_pid UUID NOT NULL,
                     stage VARCHAR NOT NULL DEFAULT 'identified',
                     readiness VARCHAR NULL,
                     added_on DATE NOT NULL DEFAULT CURRENT_DATE,
                     deleted_at TIMESTAMPTZ NULL,
                     UNIQUE (pipeline_pid, subject_kind, subject_pid)
                 );
                 CREATE INDEX IF NOT EXISTS pipeline_members_pipeline
                     ON pipeline_members (pipeline_pid);
                 CREATE TABLE IF NOT EXISTS early_career_programs (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     name VARCHAR NOT NULL UNIQUE,
                     kind VARCHAR NOT NULL,
                     level INTEGER NULL,
                     duration_months INTEGER NOT NULL,
                     min_off_the_job_hours INTEGER NULL,
                     provider_ref VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS early_career_programs_kind
                     ON early_career_programs (kind);
                 CREATE TABLE IF NOT EXISTS program_placements (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     program_pid UUID NOT NULL,
                     employee_pid UUID NOT NULL,
                     supervisor_pid UUID NULL,
                     started_on DATE NOT NULL,
                     ends_on DATE NULL,
                     status VARCHAR NOT NULL DEFAULT 'offered',
                     off_the_job_hours INTEGER NOT NULL DEFAULT 0,
                     outcome VARCHAR NOT NULL DEFAULT 'pending',
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS program_placements_program
                     ON program_placements (program_pid);
                 CREATE INDEX IF NOT EXISTS program_placements_employee
                     ON program_placements (employee_pid);",
            )
            .await?;
        Ok(())
    }

    /// Drop the six tables and the two `succession_plans` columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS program_placements;
                 DROP TABLE IF EXISTS early_career_programs;
                 DROP TABLE IF EXISTS pipeline_members;
                 DROP TABLE IF EXISTS talent_pipelines;
                 DROP TABLE IF EXISTS development_plan_items;
                 DROP TABLE IF EXISTS development_plans;
                 ALTER TABLE succession_plans
                     DROP COLUMN IF EXISTS risk_of_loss,
                     DROP COLUMN IF EXISTS vacancy_expected_on;",
            )
            .await?;
        Ok(())
    }
}
