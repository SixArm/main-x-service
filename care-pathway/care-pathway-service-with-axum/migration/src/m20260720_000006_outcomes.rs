//! Migration: pathway-instance **outcomes** — a recorded closure
//! `outcome` on `pathway_instances` (declared at close; nullable so
//! existing rows keep behaviour) and an `instance_measures` table for
//! recorded clinical / patient-reported measures over time (the honest
//! basis for outcome analytics — derived only from what was recorded).

use sea_orm_migration::prelude::*;

/// The outcomes migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add `outcome`; create `instance_measures`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE pathway_instances ADD COLUMN IF NOT EXISTS outcome VARCHAR NULL;
                 CREATE TABLE IF NOT EXISTS instance_measures (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     instance_pid UUID NOT NULL,
                     name VARCHAR NOT NULL,
                     value_numeric DOUBLE PRECISION NULL,
                     value_text VARCHAR NULL,
                     unit VARCHAR NULL,
                     recorded_on DATE NOT NULL DEFAULT CURRENT_DATE
                 );
                 CREATE INDEX IF NOT EXISTS instance_measures_instance
                     ON instance_measures (instance_pid, name, recorded_on);",
            )
            .await?;
        Ok(())
    }

    /// Drop the table + column.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS instance_measures;
                 ALTER TABLE pathway_instances DROP COLUMN IF EXISTS outcome;",
            )
            .await?;
        Ok(())
    }
}
