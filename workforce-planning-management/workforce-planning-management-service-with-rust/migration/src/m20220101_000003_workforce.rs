//! Migration: the workforce-management tables (WPM-R4–R6) —
//! `time_entries` (whole minutes), `leave_entitlements` +
//! `leave_requests` (whole days), `shifts` + `shift_assignments`.

use sea_orm_migration::prelude::*;

/// The workforce-tables migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the five workforce tables + lookup indexes.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS time_entries (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 employee_pid UUID NOT NULL,
                 worked_on DATE NOT NULL,
                 minutes INTEGER NOT NULL,
                 kind VARCHAR NOT NULL,
                 status VARCHAR NOT NULL,
                 notes VARCHAR NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS time_entries_employee_day \
             ON time_entries (employee_pid, worked_on)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS leave_entitlements (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 employee_pid UUID NOT NULL,
                 kind VARCHAR NOT NULL,
                 year INTEGER NOT NULL,
                 entitled_days INTEGER NOT NULL,
                 used_days INTEGER NOT NULL DEFAULT 0,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS leave_entitlements_key \
             ON leave_entitlements (employee_pid, kind, year) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS leave_requests (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 employee_pid UUID NOT NULL,
                 kind VARCHAR NOT NULL,
                 start_on DATE NOT NULL,
                 end_on DATE NOT NULL,
                 days INTEGER NOT NULL,
                 status VARCHAR NOT NULL,
                 negative_balance BOOLEAN NOT NULL DEFAULT FALSE,
                 reason VARCHAR NULL,
                 decided_by VARCHAR NULL,
                 decided_at TIMESTAMPTZ NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS leave_requests_employee \
             ON leave_requests (employee_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS shifts (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 department VARCHAR NOT NULL,
                 starts_at TIMESTAMPTZ NOT NULL,
                 ends_at TIMESTAMPTZ NOT NULL,
                 required_headcount INTEGER NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS shifts_department ON shifts (department, starts_at)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS shift_assignments (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 shift_pid UUID NOT NULL,
                 employee_pid UUID NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS shift_assignments_employee \
             ON shift_assignments (employee_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the workforce tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for table in [
            "shift_assignments",
            "shifts",
            "leave_requests",
            "leave_entitlements",
            "time_entries",
        ] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
