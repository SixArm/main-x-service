//! Migration: the assessment tables — an `assessment_instruments`
//! catalog (a named test, its category, and the scales it reports),
//! per-subject `assessments` (one administration to one candidate or
//! employee, optionally tied to an application), and the per-scale
//! `assessment_results`.
//!
//! Scores are integers throughout: percentiles are 0–100 and raw
//! scores are whole points out of a whole maximum, so no floats reach
//! the schema. Results are sensitive personal data — they profile
//! cognition and behaviour — so reads are masked and audited at the
//! controller; deletion is soft so the trail stays intact.

use sea_orm_migration::prelude::*;

/// The assessments migration (name derived from the module).
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
                "CREATE TABLE IF NOT EXISTS assessment_instruments (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     name VARCHAR NOT NULL UNIQUE,
                     category VARCHAR NOT NULL,
                     provider VARCHAR NULL,
                     scales JSONB NOT NULL DEFAULT '[]'::jsonb,
                     duration_minutes INTEGER NULL,
                     validity_months INTEGER NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS assessment_instruments_category
                     ON assessment_instruments (category);
                 CREATE TABLE IF NOT EXISTS assessments (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     instrument_pid UUID NOT NULL,
                     subject_kind VARCHAR NOT NULL,
                     subject_pid UUID NOT NULL,
                     application_pid UUID NULL,
                     status VARCHAR NOT NULL DEFAULT 'scheduled',
                     scheduled_on DATE NULL,
                     completed_on DATE NULL,
                     expires_on DATE NULL,
                     administered_by VARCHAR NULL,
                     notes VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS assessments_subject
                     ON assessments (subject_kind, subject_pid);
                 CREATE INDEX IF NOT EXISTS assessments_application
                     ON assessments (application_pid);
                 CREATE INDEX IF NOT EXISTS assessments_instrument
                     ON assessments (instrument_pid);
                 CREATE TABLE IF NOT EXISTS assessment_results (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     assessment_pid UUID NOT NULL,
                     scale VARCHAR NOT NULL,
                     raw_score INTEGER NULL,
                     max_score INTEGER NULL,
                     percentile INTEGER NULL,
                     band VARCHAR NULL,
                     narrative VARCHAR NULL,
                     UNIQUE (assessment_pid, scale)
                 );
                 CREATE INDEX IF NOT EXISTS assessment_results_assessment
                     ON assessment_results (assessment_pid);",
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
                "DROP TABLE IF EXISTS assessment_results;
                 DROP TABLE IF EXISTS assessments;
                 DROP TABLE IF EXISTS assessment_instruments;",
            )
            .await?;
        Ok(())
    }
}
