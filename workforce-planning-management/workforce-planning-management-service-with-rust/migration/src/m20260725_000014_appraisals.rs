//! Migration: 360° appraisals (WPM-R29) — the appraisal (subject +
//! declared competencies + lifecycle), its rater nominations, and the
//! per-rater responses. The response row links to its nomination **by
//! design** (WPM-D21: procedural anonymity — the link enforces
//! one-response-per-rater and completion tracking; the API never
//! discloses rater-level content).

use sea_orm_migration::prelude::*;

/// The appraisals migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the three tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS appraisals (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     employee_pid UUID NOT NULL,
                     competencies JSONB NOT NULL DEFAULT '[]',
                     status VARCHAR NOT NULL DEFAULT 'draft',
                     shared_on DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS appraisals_employee
                     ON appraisals (employee_pid);
                 CREATE TABLE IF NOT EXISTS appraisal_nominations (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     appraisal_pid UUID NOT NULL,
                     rater_pid UUID NOT NULL,
                     rater_group VARCHAR NOT NULL,
                     UNIQUE (appraisal_pid, rater_pid)
                 );
                 CREATE INDEX IF NOT EXISTS appraisal_nominations_appraisal
                     ON appraisal_nominations (appraisal_pid);
                 CREATE TABLE IF NOT EXISTS appraisal_responses (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     appraisal_pid UUID NOT NULL,
                     nomination_pid UUID NOT NULL UNIQUE,
                     rater_group VARCHAR NOT NULL,
                     scores JSONB NOT NULL DEFAULT '{}',
                     comment VARCHAR NULL
                 );
                 CREATE INDEX IF NOT EXISTS appraisal_responses_appraisal
                     ON appraisal_responses (appraisal_pid);",
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
                "DROP TABLE IF EXISTS appraisal_responses;
                 DROP TABLE IF EXISTS appraisal_nominations;
                 DROP TABLE IF EXISTS appraisals;",
            )
            .await?;
        Ok(())
    }
}
