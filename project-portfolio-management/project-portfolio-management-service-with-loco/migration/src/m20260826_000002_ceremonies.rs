//! Migration: **sprint ceremonies** and the **commitment snapshot**
//! (entity spec §5.9.3 / FR-29).
//!
//! `sprints` and `sprint_notes` already exist — the retrospective, in
//! effect. This adds the other three ceremonies as records and, more
//! importantly, the commitment snapshot.
//!
//! ## Why a commitment snapshot is a table
//!
//! Planning declares a set of tasks at sprint start. Without recording
//! it, scope added mid-sprint is indistinguishable from scope committed
//! at the outset — the sprint simply appears to have been larger all
//! along. A snapshot makes a later change read as **a change**, rather
//! than as a moved goalpost.
//!
//! It is written once per sprint and never updated: re-committing would
//! defeat the purpose. Re-planning means a new sprint.

use sea_orm_migration::prelude::*;

/// The ceremonies migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `ceremonies` and `sprint_commitments`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS ceremonies (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     sprint_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL
                         CHECK (kind IN ('planning', 'daily', 'review', 'retrospective')),
                     held_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     facilitator_ref VARCHAR NULL,
                     summary VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS ceremonies_sprint
                     ON ceremonies (sprint_pid, held_at);
                 -- One planning and one review per sprint: a second of
                 -- either is a re-plan, which is a new sprint.
                 CREATE UNIQUE INDEX IF NOT EXISTS ceremonies_one_planning
                     ON ceremonies (sprint_pid, kind)
                     WHERE deleted_at IS NULL AND kind IN ('planning', 'review');

                 CREATE TABLE IF NOT EXISTS sprint_commitments (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     sprint_pid UUID NOT NULL,
                     task_pid UUID NOT NULL,
                     committed_at TIMESTAMPTZ NOT NULL DEFAULT now()
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS sprint_commitments_pair
                     ON sprint_commitments (sprint_pid, task_pid);",
            )
            .await?;
        Ok(())
    }

    /// Drop both tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS sprint_commitments;
                 DROP TABLE IF EXISTS ceremonies;",
            )
            .await?;
        Ok(())
    }
}
