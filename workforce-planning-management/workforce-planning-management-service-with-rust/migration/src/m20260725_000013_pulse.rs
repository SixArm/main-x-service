//! Migration: the anonymous wellbeing pulse (WPM-R28) — surveys and
//! their responses. The response table is **anonymous by
//! construction** (WPM-D20): it carries the survey, the department,
//! the score, and the date — there is deliberately no author column,
//! and no hash that could stand in for one.

use sea_orm_migration::prelude::*;

/// The pulse migration (name derived from the module).
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
                "CREATE TABLE IF NOT EXISTS pulse_surveys (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     name VARCHAR NOT NULL UNIQUE,
                     question VARCHAR NOT NULL,
                     active_from DATE NULL,
                     active_until DATE NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE TABLE IF NOT EXISTS pulse_responses (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     survey_pid UUID NOT NULL,
                     department VARCHAR NOT NULL,
                     score INTEGER NOT NULL,
                     submitted_on DATE NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS pulse_responses_survey
                     ON pulse_responses (survey_pid);",
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
                "DROP TABLE IF EXISTS pulse_responses;
                 DROP TABLE IF EXISTS pulse_surveys;",
            )
            .await?;
        Ok(())
    }
}
