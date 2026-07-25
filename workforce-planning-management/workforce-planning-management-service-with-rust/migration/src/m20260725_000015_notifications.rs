//! Migration: in-app notifications (WPM-R31 / WPM-D23) — rows written
//! by WPM's own lifecycle transitions, reference-only (never scores,
//! comments, or masked-tier values), owned by one employee.

use sea_orm_migration::prelude::*;

/// The notifications migration (name derived from the module).
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
                "CREATE TABLE IF NOT EXISTS notifications (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     employee_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL,
                     body VARCHAR NOT NULL,
                     data JSONB NOT NULL DEFAULT '{}',
                     read_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS notifications_employee
                     ON notifications (employee_pid, read_at);",
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
            .execute_unprepared("DROP TABLE IF EXISTS notifications;")
            .await?;
        Ok(())
    }
}
