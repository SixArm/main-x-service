//! Migration: ergonomic (DSE) workstation assessments (WPM-R32) — an
//! assessment per employee + workstation with its checklist items.
//! About the workstation, never the body (WPM-D24): there is no
//! symptom or health column, and the note field is for equipment.

use sea_orm_migration::prelude::*;

/// The ergonomics migration (name derived from the module).
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
                "CREATE TABLE IF NOT EXISTS ergonomic_assessments (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     employee_pid UUID NOT NULL,
                     workstation VARCHAR NOT NULL,
                     status VARCHAR NOT NULL DEFAULT 'open',
                     assessed_on DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS ergonomic_assessments_employee
                     ON ergonomic_assessments (employee_pid);
                 CREATE TABLE IF NOT EXISTS ergonomic_items (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     assessment_pid UUID NOT NULL,
                     name VARCHAR NOT NULL,
                     ok BOOLEAN NULL,
                     note VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS ergonomic_items_assessment
                     ON ergonomic_items (assessment_pid);",
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
                "DROP TABLE IF EXISTS ergonomic_items;
                 DROP TABLE IF EXISTS ergonomic_assessments;",
            )
            .await?;
        Ok(())
    }
}
