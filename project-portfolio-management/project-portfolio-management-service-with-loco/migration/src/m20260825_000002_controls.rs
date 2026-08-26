//! Migration: **controls** — the Controlling-process register (entity
//! spec §5.9.8 / FR-38, FR-39): `controls`, `control_readings`, and
//! `control_actions`.
//!
//! ## Why readings are append-only
//!
//! `control_readings` has no update or delete path in this service, for
//! the same reason `task_transitions` has none: a control history that
//! can be rewritten measures whatever the editor wanted. Correcting a
//! reading means recording another one.
//!
//! ## Why the timing is constrained in the schema
//!
//! `timing` decides what a *failing* control may do — feedforward may
//! block a write, concurrent may only warn, feedback may only record
//! (`crate::controls::permitted_response`). A row carrying a timing
//! outside that set would have no defined response, so the CHECK is in
//! the schema rather than only in the handler.
//!
//! `verdict` is constrained for the sharper reason that `unmeasured`
//! must remain a **third** value: if it could be spelled anything, a
//! typo would fall through whatever the read path treats as "not a
//! fail", and a control nobody measured would quietly read as passing.

use sea_orm_migration::prelude::*;

/// The controls migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the three control tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS controls (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     plan_pid UUID NOT NULL,
                     name VARCHAR NOT NULL,
                     timing VARCHAR NOT NULL
                         CHECK (timing IN ('feedforward', 'concurrent', 'feedback')),
                     metric VARCHAR NOT NULL,
                     target_value BIGINT NOT NULL,
                     comparator VARCHAR NOT NULL
                         CHECK (comparator IN ('at_least', 'at_most', 'within', 'equals')),
                     tolerance BIGINT NULL CHECK (tolerance IS NULL OR tolerance >= 0),
                     unit VARCHAR NULL,
                     currency VARCHAR NULL,
                     source_kind VARCHAR NOT NULL
                         CHECK (source_kind IN ('metric', 'query', 'manual')),
                     source_ref VARCHAR NULL,
                     cadence_days BIGINT NULL CHECK (cadence_days IS NULL OR cadence_days > 0),
                     owner_ref VARCHAR NULL,
                     enabled BOOLEAN NOT NULL DEFAULT true,
                     deleted_at TIMESTAMPTZ NULL,
                     -- A `within` control needs a band; without one the
                     -- comparison has nothing to compare against.
                     CONSTRAINT controls_within_needs_tolerance
                         CHECK (comparator <> 'within' OR tolerance IS NOT NULL)
                 );
                 CREATE INDEX IF NOT EXISTS controls_plan ON controls (plan_pid);
                 CREATE INDEX IF NOT EXISTS controls_timing ON controls (timing);

                 CREATE TABLE IF NOT EXISTS control_readings (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     control_pid UUID NOT NULL,
                     observed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     -- NULL is `unmeasured`, which is a third verdict
                     -- and never a pass.
                     value BIGINT NULL,
                     verdict VARCHAR NOT NULL
                         CHECK (verdict IN ('pass', 'fail', 'unmeasured')),
                     gap BIGINT NULL,
                     method VARCHAR NOT NULL,
                     -- An explicit acceptance of a failing reading. A
                     -- failure with neither this nor an action is
                     -- reported as unanswered.
                     accepted_at TIMESTAMPTZ NULL,
                     accepted_reason VARCHAR NULL
                 );
                 CREATE INDEX IF NOT EXISTS control_readings_control
                     ON control_readings (control_pid, observed_at DESC);

                 CREATE TABLE IF NOT EXISTS control_actions (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     reading_pid UUID NOT NULL,
                     kind VARCHAR NOT NULL
                         CHECK (kind IN ('correct', 'adjust', 'retrain', 'accept', 'escalate')),
                     description VARCHAR NOT NULL,
                     owner_ref VARCHAR NULL,
                     due_date DATE NULL,
                     -- Actions convert into the work stores that already
                     -- exist rather than becoming a fifth one.
                     converted_task_pid UUID NULL,
                     converted_issue_pid UUID NULL,
                     closed_at TIMESTAMPTZ NULL,
                     outcome VARCHAR NULL,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS control_actions_reading
                     ON control_actions (reading_pid);",
            )
            .await?;
        Ok(())
    }

    /// Drop the three tables, children first.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS control_actions;
                 DROP TABLE IF EXISTS control_readings;
                 DROP TABLE IF EXISTS controls;",
            )
            .await?;
        Ok(())
    }
}
