//! Migration: the **project phase** — a denormalised `plans.phase`
//! column plus the append-only `phase_transitions` log (entity spec
//! §5.9.4 / FR-30).
//!
//! ## Why both a column and a log
//!
//! The phase itself rides in the JSONB payload (it is a field on the
//! matcher's `Plan`, informational-only and never scored — the same
//! posture as `status`). The **column** is a denormalised projection so
//! a list or funnel read does not have to open every payload, exactly
//! as `name` and `parent_pid` already are.
//!
//! The **log** is what makes per-phase *duration* measurable rather
//! than only the current value. Without it, the moment a plan moves
//! twice the first interval is gone — the same reason
//! `task_transitions` exists.
//!
//! ## Append-only
//!
//! No update or delete path, matching `task_transitions` and
//! `control_readings`: a phase history that can be rewritten cannot
//! support a duration claim.
//!
//! ## No backfill
//!
//! Unlike the time-based-analysis migration, nothing is synthesised
//! here. Every existing plan has **no** phase, which is the truth —
//! inventing `initiating` for a plan already in delivery would be a
//! fabricated history, and a labelled backfill would still be a
//! measurement of nothing. Plans acquire a phase when an operator sets
//! one.

use sea_orm_migration::prelude::*;

/// The phase-transition migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add `plans.phase` and create `phase_transitions`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE plans ADD COLUMN IF NOT EXISTS phase VARCHAR NULL
                     CHECK (phase IS NULL OR phase IN
                         ('initiating', 'planning', 'executing', 'controlling', 'closing'));
                 CREATE INDEX IF NOT EXISTS plans_phase ON plans (phase);

                 CREATE TABLE IF NOT EXISTS phase_transitions (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     -- NULL marks the plan's first phase.
                     from_phase VARCHAR NULL
                         CHECK (from_phase IS NULL OR from_phase IN
                             ('initiating', 'planning', 'executing', 'controlling', 'closing')),
                     to_phase VARCHAR NOT NULL
                         CHECK (to_phase IN
                             ('initiating', 'planning', 'executing', 'controlling', 'closing')),
                     occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     actor VARCHAR NULL,
                     -- Required by the handler for a backward move: a
                     -- regression is normal, an unexplained one is not.
                     reason VARCHAR NULL
                 );
                 CREATE INDEX IF NOT EXISTS phase_transitions_plan
                     ON phase_transitions (plan_pid, occurred_at);",
            )
            .await?;
        Ok(())
    }

    /// Drop the log and the column.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS phase_transitions;
                 ALTER TABLE plans DROP COLUMN IF EXISTS phase;",
            )
            .await?;
        Ok(())
    }
}
