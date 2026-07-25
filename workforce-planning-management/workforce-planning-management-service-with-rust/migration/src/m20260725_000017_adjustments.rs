//! Migration: reasonable adjustments (WPM-R33 / WPM-D25) — the
//! barrier, its impact, and the change that would reduce it. There is
//! deliberately **no diagnosis, condition, or medical-evidence
//! column**: the schema cannot hold what must never be required.

use sea_orm_migration::prelude::*;

/// The adjustments migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS adjustment_requests (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     employee_pid UUID NOT NULL,
                     category VARCHAR NOT NULL,
                     barrier VARCHAR NOT NULL,
                     impact VARCHAR NOT NULL,
                     adjustment VARCHAR NOT NULL,
                     status VARCHAR NOT NULL DEFAULT 'requested',
                     decision_note VARCHAR NULL,
                     decided_on DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS adjustment_requests_employee
                     ON adjustment_requests (employee_pid);",
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
            .execute_unprepared("DROP TABLE IF EXISTS adjustment_requests;")
            .await?;
        Ok(())
    }
}
