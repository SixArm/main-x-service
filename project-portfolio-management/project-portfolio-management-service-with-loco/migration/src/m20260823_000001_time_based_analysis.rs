//! Migration: **time-based analysis** (TBA) — the durable task
//! transition log.
//!
//! `tasks` carried only `status_changed_at` (when the *current* status
//! began) and `done_at`. The moment a task moved twice, the first
//! interval was gone — so the one question time-based analysis exists
//! to answer, *of the time this took, how much was somebody actually
//! working on it?*, could not be asked at all. See
//! `spec/time-based-analysis.md` §5.
//!
//! The log is **append-only** by design (no edit or delete endpoint
//! exists): an editable flow log measures whatever the editor wanted.
//!
//! ## The backfill is labelled, not hidden
//!
//! There is no history to recover, so this migration writes **one
//! synthetic transition per live task** (`NULL → current status` at
//! `status_changed_at`) and flags it `backfilled = true`. That makes an
//! existing board analysable immediately, and every analysis reports
//! how many of its transitions were synthesised — so a figure resting
//! on invented history is visibly weaker than one resting on observed
//! moves. Writing the backfill without the flag would have been the
//! same code and a lie.

use sea_orm_migration::prelude::*;

/// The time-based-analysis migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `task_transitions` and seed the labelled backfill.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS task_transitions (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     task_pid UUID NOT NULL,
                     plan_pid UUID NOT NULL,
                     from_status VARCHAR NULL,
                     to_status VARCHAR NOT NULL,
                     at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     actor_ref VARCHAR NULL,
                     assignee_ref VARCHAR NULL,
                     backfilled BOOLEAN NOT NULL DEFAULT false
                 );
                 -- The per-task read: derive one item's intervals in order.
                 CREATE INDEX IF NOT EXISTS task_transitions_task
                     ON task_transitions (task_pid, at);
                 -- The plan-wide read: `plan_pid` is denormalised so a
                 -- whole board is one query rather than a join per task.
                 CREATE INDEX IF NOT EXISTS task_transitions_plan
                     ON task_transitions (plan_pid, at);
                 CREATE INDEX IF NOT EXISTS task_transitions_status
                     ON task_transitions (to_status);
                 -- The labelled backfill: one synthetic transition per
                 -- live task, only where the task has none already, so
                 -- re-running the migration cannot duplicate it.
                 INSERT INTO task_transitions
                     (pid, task_pid, plan_pid, from_status, to_status, at, backfilled)
                 SELECT gen_random_uuid(), t.pid, t.plan_pid, NULL,
                        t.status, t.status_changed_at, true
                 FROM tasks t
                 WHERE t.deleted_at IS NULL
                   AND NOT EXISTS (
                       SELECT 1 FROM task_transitions x WHERE x.task_pid = t.pid
                   );",
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
            .execute_unprepared("DROP TABLE IF EXISTS task_transitions;")
            .await?;
        Ok(())
    }
}
