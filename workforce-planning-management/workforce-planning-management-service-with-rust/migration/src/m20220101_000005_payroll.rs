//! Migration: the payroll & compensation tables (HCM-R13, HCM-R14) —
//! `payroll_runs`, `payslips` (minor units; deductions as JSONB
//! lines), `benchmarks`.

use sea_orm_migration::prelude::*;

/// The payroll-tables migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the three payroll tables + lookup indexes.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS payroll_runs (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 organization_ref VARCHAR NOT NULL,
                 period_start DATE NOT NULL,
                 period_end DATE NOT NULL,
                 status VARCHAR NOT NULL,
                 approved_by VARCHAR NULL,
                 notes VARCHAR NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS payslips (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 run_pid UUID NOT NULL,
                 employee_pid UUID NOT NULL,
                 currency VARCHAR NOT NULL,
                 gross_minor BIGINT NOT NULL,
                 deductions JSONB NOT NULL,
                 net_minor BIGINT NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS payslips_run ON payslips (run_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS payslips_employee ON payslips (employee_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS benchmarks (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 job_title VARCHAR NOT NULL,
                 currency VARCHAR NOT NULL,
                 min_minor BIGINT NOT NULL,
                 median_minor BIGINT NOT NULL,
                 max_minor BIGINT NOT NULL,
                 source VARCHAR NOT NULL,
                 as_of DATE NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        Ok(())
    }

    /// Drop the payroll tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for table in ["benchmarks", "payslips", "payroll_runs"] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
