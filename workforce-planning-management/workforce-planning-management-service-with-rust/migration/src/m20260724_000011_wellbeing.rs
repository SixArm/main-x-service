//! Migration: the wellbeing tables (WPM-R25) — configurable
//! **health-entitlement rules** (e.g. NHS vaccination cohorts) and the
//! per-employee **acknowledgements** of their prompts.
//!
//! The rule row can hold only non-clinical predicates — an age band and
//! department / job-title lists (WPM-D17): there is deliberately no
//! column a health-status cohort could be expressed in. The
//! acknowledgement row records the employee's response to a prompt
//! (`booked | done | declined | dismissed`) — an HR workflow fact,
//! never a vaccination status.

use sea_orm_migration::prelude::*;

/// The wellbeing migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the two tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS wellbeing_entitlements (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     name VARCHAR NOT NULL UNIQUE,
                     description VARCHAR NOT NULL,
                     info_url VARCHAR NULL,
                     min_age INTEGER NULL,
                     max_age INTEGER NULL,
                     departments JSONB NOT NULL DEFAULT '[]',
                     job_titles JSONB NOT NULL DEFAULT '[]',
                     doses INTEGER NOT NULL DEFAULT 1,
                     active_from DATE NULL,
                     active_until DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE TABLE IF NOT EXISTS entitlement_acknowledgements (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     entitlement_pid UUID NOT NULL,
                     employee_pid UUID NOT NULL,
                     response VARCHAR NOT NULL,
                     responded_on DATE NOT NULL,
                     reminded_on DATE NULL,
                     UNIQUE (entitlement_pid, employee_pid)
                 );
                 CREATE INDEX IF NOT EXISTS entitlement_acks_employee
                     ON entitlement_acknowledgements (employee_pid);
                 CREATE INDEX IF NOT EXISTS entitlement_acks_entitlement
                     ON entitlement_acknowledgements (entitlement_pid);",
            )
            .await?;
        Ok(())
    }

    /// Drop the two tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS entitlement_acknowledgements;
                 DROP TABLE IF EXISTS wellbeing_entitlements;",
            )
            .await?;
        Ok(())
    }
}
