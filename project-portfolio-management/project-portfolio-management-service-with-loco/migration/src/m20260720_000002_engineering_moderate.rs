//! Migration: the engineering moderate fits — task story `points`
//! (team-local; velocity derives from them), `sprint_notes` (retro /
//! feedback log per sprint, convertible to tasks), and
//! `devops_events` (ingested deploy / incident / recovery events —
//! the only source the DORA-style metrics ever derive from).

use sea_orm_migration::prelude::*;

/// The engineering-moderate migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add `tasks.points`; create `sprint_notes` + `devops_events`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE tasks ADD COLUMN IF NOT EXISTS points INTEGER NULL;
                 CREATE TABLE IF NOT EXISTS sprint_notes (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     sprint_pid UUID NOT NULL,
                     category VARCHAR NOT NULL,
                     body VARCHAR NOT NULL,
                     task_pid UUID NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS sprint_notes_sprint
                     ON sprint_notes (sprint_pid);
                 CREATE TABLE IF NOT EXISTS devops_events (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL,
                     environment VARCHAR NULL,
                     version VARCHAR NULL,
                     reference VARCHAR NULL,
                     incident_pid UUID NULL,
                     caused_by_deploy_pid UUID NULL,
                     occurred_at TIMESTAMPTZ NOT NULL DEFAULT now()
                 );
                 CREATE INDEX IF NOT EXISTS devops_events_kind
                     ON devops_events (kind, occurred_at);",
            )
            .await?;
        Ok(())
    }

    /// Drop the tables + column.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS devops_events;
                 DROP TABLE IF EXISTS sprint_notes;
                 ALTER TABLE tasks DROP COLUMN IF EXISTS points;",
            )
            .await?;
        Ok(())
    }
}
