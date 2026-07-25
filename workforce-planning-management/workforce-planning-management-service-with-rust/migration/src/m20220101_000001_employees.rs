//! Migration: create the `employees` table — the single source of
//! employment truth (WPM-R7). Identities are `EntityRef` URNs; the
//! employee number is unique per organization; salary is minor units
//! (sensitive — masked at the read surface).
//!
//! Written as explicit SQL (family lesson: the loco `create_table`
//! helper pluralizes names; explicit SQL also gives BIGINT/DATE
//! control).

use sea_orm_migration::prelude::*;

/// The `employees` migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `employees` + its uniqueness and lookup indexes.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS employees (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 person_ref VARCHAR NOT NULL,
                 worker_ref VARCHAR NULL,
                 organization_ref VARCHAR NOT NULL,
                 employee_number VARCHAR NOT NULL,
                 display_name VARCHAR NOT NULL,
                 status VARCHAR NOT NULL,
                 employment_type VARCHAR NOT NULL,
                 fte_percent INTEGER NOT NULL,
                 department VARCHAR NOT NULL,
                 job_title VARCHAR NOT NULL,
                 manager_pid UUID NULL,
                 salary_minor BIGINT NULL,
                 salary_currency VARCHAR NULL,
                 hired_on DATE NOT NULL,
                 terminated_on DATE NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        // The per-organization employee-number uniqueness (WPM-R7),
        // scoped to live rows so a re-used number after termination +
        // soft delete stays possible.
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS employees_org_number \
             ON employees (organization_ref, employee_number) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS employees_department ON employees (department)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS employees_manager ON employees (manager_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop `employees` (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS employees")
            .await?;
        Ok(())
    }
}
