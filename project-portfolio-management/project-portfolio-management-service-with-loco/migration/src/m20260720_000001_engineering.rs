//! Migration: the engineering-team tables — `tasks` (the spec-§13
//! operational sub-resource behind the Kanban board, with status /
//! completion stamps for honest flow metrics) and `sprints`
//! (time-boxed iterations behind the burndown view) — plus the
//! milestone `kind` column (demo / release / checkpoint calendar).

use sea_orm_migration::prelude::*;

/// The engineering migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `tasks` + `sprints`; add `milestones.kind`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS sprints (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     name VARCHAR NOT NULL,
                     starts_on DATE NOT NULL,
                     ends_on DATE NOT NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS sprints_item ON sprints (plan_pid);
                 CREATE TABLE IF NOT EXISTS tasks (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     sprint_pid UUID NULL,
                     title VARCHAR NOT NULL,
                     description VARCHAR NULL,
                     status VARCHAR NOT NULL DEFAULT 'todo',
                     assignee_ref VARCHAR NULL,
                     status_changed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     done_at TIMESTAMPTZ NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS tasks_item ON tasks (plan_pid);
                 CREATE INDEX IF NOT EXISTS tasks_status ON tasks (status);
                 ALTER TABLE milestones ADD COLUMN IF NOT EXISTS kind VARCHAR NULL;",
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
                "DROP TABLE IF EXISTS tasks;
                 DROP TABLE IF EXISTS sprints;
                 ALTER TABLE milestones DROP COLUMN IF EXISTS kind;",
            )
            .await?;
        Ok(())
    }
}
