//! Migration: **`milestone_due` trigger claim table** (entity spec
//! FR-32, repo `tasks.md` PRO-P20 / T-21). The first of the three
//! remaining triggers ("a date arriving") — scoped narrowly to
//! `milestones.due`, the one dated field in this service with an
//! unambiguous "arrived" reading (see this crate's own `spec/13-tasks.md`
//! T-21 for why the other two, field-change and SLE-breach, are not
//! attempted in the same slice).
//!
//! ## Why a claim table, not a status column on `milestones`
//!
//! `task_moved` / `plan_phase_changed` / `plan_stage_changed` /
//! `review_submitted` all fire off a **write that already happened**:
//! the event is inherently one-shot, so `fire()` just evaluates the
//! rules once, synchronously. A due date is not an event — it is a
//! **condition that stays true** every time a sweep looks at it, so
//! firing needs its own **exactly-once claim**, mirroring
//! `scheduled_actions`' `pending → fired` conditional update (FR-16d)
//! but shaped differently because the thing arriving (a milestone) is
//! not itself the row that tracks whether it fired — a milestone can be
//! matched by more than one enabled rule, and each rule's firing is
//! independent.
//!
//! `automation_milestone_fires (automation_pid, milestone_pid)` is that
//! claim: the sweep attempts an `INSERT … ON CONFLICT DO NOTHING
//! RETURNING id`, and only a caller that actually inserted a row (zero
//! rows back means someone already claimed it) proceeds to apply the
//! rule's actions. A milestone re-opened after being marked done and
//! then re-completed does not refire — the claim persists for the life
//! of the (rule, milestone) pair, matching "exactly once" rather than
//! "once per time the condition holds".

use sea_orm_migration::prelude::*;

/// The milestone-due claim-table migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `automation_milestone_fires`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS automation_milestone_fires (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     automation_pid UUID NOT NULL,
                     milestone_pid UUID NOT NULL,
                     UNIQUE (automation_pid, milestone_pid)
                 );
                 CREATE INDEX IF NOT EXISTS automation_milestone_fires_milestone
                     ON automation_milestone_fires (milestone_pid);",
            )
            .await?;
        Ok(())
    }

    /// Drop `automation_milestone_fires`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS automation_milestone_fires;")
            .await?;
        Ok(())
    }
}
