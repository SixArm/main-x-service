//! Migration: the collaboration / automation capability tables —
//! `reviews` (delegate an idea, proposal, or plan to the right internal
//! or external expert), `automations` + `automation_runs` (the rules
//! fired when an item moves across the Kanban board, and the honest
//! log of what each firing actually did), `scheduled_actions` (the
//! set-and-forget deadline queue), and `notifications` (the in-app
//! inbox those two write into — there is no email transport here).
//!
//! No Smart Score table: the score is **derived** on read from facts
//! the service already stores (see `src/prioritisation.rs`), so it can
//! never drift from the data it claims to summarise.

use sea_orm_migration::prelude::*;

/// The capabilities migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `reviews`, `automations`, `automation_runs`,
    /// `scheduled_actions`, and `notifications`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS reviews (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     subject_kind VARCHAR NOT NULL,
                     subject_pid UUID NOT NULL,
                     reviewer_ref VARCHAR NOT NULL,
                     reviewer_scope VARCHAR NOT NULL DEFAULT 'internal',
                     expertise VARCHAR NULL,
                     status VARCHAR NOT NULL DEFAULT 'invited',
                     due_on DATE NULL,
                     score INTEGER NULL,
                     recommendation VARCHAR NULL,
                     comment VARCHAR NULL,
                     invited_by VARCHAR NULL,
                     responded_at TIMESTAMPTZ NULL,
                     submitted_at TIMESTAMPTZ NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS reviews_subject
                     ON reviews (subject_kind, subject_pid);
                 CREATE INDEX IF NOT EXISTS reviews_reviewer ON reviews (reviewer_ref);
                 CREATE INDEX IF NOT EXISTS reviews_status ON reviews (status);
                 -- One live invitation per (subject, reviewer): re-inviting the
                 -- same expert refreshes that row instead of stacking duplicates.
                 CREATE UNIQUE INDEX IF NOT EXISTS reviews_subject_reviewer_live
                     ON reviews (subject_kind, subject_pid, reviewer_ref)
                     WHERE deleted_at IS NULL;

                 CREATE TABLE IF NOT EXISTS automations (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NULL,
                     name VARCHAR NOT NULL,
                     trigger_kind VARCHAR NOT NULL,
                     from_status VARCHAR NULL,
                     to_status VARCHAR NULL,
                     action_kind VARCHAR NOT NULL,
                     action_value JSONB NOT NULL DEFAULT '{}'::jsonb,
                     enabled BOOLEAN NOT NULL DEFAULT true,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS automations_plan ON automations (plan_pid);
                 CREATE INDEX IF NOT EXISTS automations_trigger
                     ON automations (trigger_kind, enabled);

                 CREATE TABLE IF NOT EXISTS automation_runs (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     automation_pid UUID NOT NULL,
                     subject_kind VARCHAR NOT NULL,
                     subject_pid UUID NOT NULL,
                     outcome VARCHAR NOT NULL,
                     detail JSONB NOT NULL DEFAULT '{}'::jsonb
                 );
                 CREATE INDEX IF NOT EXISTS automation_runs_automation
                     ON automation_runs (automation_pid);
                 CREATE INDEX IF NOT EXISTS automation_runs_subject
                     ON automation_runs (subject_kind, subject_pid);

                 CREATE TABLE IF NOT EXISTS scheduled_actions (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     subject_kind VARCHAR NOT NULL,
                     subject_pid UUID NOT NULL,
                     action_kind VARCHAR NOT NULL,
                     payload JSONB NOT NULL DEFAULT '{}'::jsonb,
                     due_at TIMESTAMPTZ NOT NULL,
                     status VARCHAR NOT NULL DEFAULT 'pending',
                     source_automation_pid UUID NULL,
                     created_by VARCHAR NULL,
                     fired_at TIMESTAMPTZ NULL,
                     outcome VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS scheduled_actions_due
                     ON scheduled_actions (status, due_at);
                 CREATE INDEX IF NOT EXISTS scheduled_actions_subject
                     ON scheduled_actions (subject_kind, subject_pid);

                 CREATE TABLE IF NOT EXISTS notifications (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     recipient_ref VARCHAR NOT NULL,
                     subject_kind VARCHAR NOT NULL,
                     subject_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL,
                     message VARCHAR NOT NULL,
                     read_at TIMESTAMPTZ NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS notifications_recipient
                     ON notifications (recipient_ref, read_at);",
            )
            .await?;
        Ok(())
    }

    /// Drop the capability tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS notifications;
                 DROP TABLE IF EXISTS scheduled_actions;
                 DROP TABLE IF EXISTS automation_runs;
                 DROP TABLE IF EXISTS automations;
                 DROP TABLE IF EXISTS reviews;",
            )
            .await?;
        Ok(())
    }
}
