//! Migration: **custom workflows** (entity spec §5.9.1 / FR-26) —
//! `workflows`, `workflow_states`, `workflow_transitions`.
//!
//! ## The category is `NOT NULL` with a CHECK, deliberately
//!
//! The board, the burndown, the timeline and every time-based-analysis
//! figure compute from what a state *means*, not from its name. An
//! uncategorised state is one the flow-efficiency denominator cannot
//! account for and the burndown cannot call finished.
//!
//! Enforcing it in the schema rather than only in the handler is the
//! point: a rule that lives in a handler is a rule until somebody
//! writes SQL, and this one is load-bearing for four separate views.
//!
//! ## Nothing is seeded
//!
//! The built-in vocabularies stay **code** defaults
//! (`crate::workflow::built_in_task`), not rows. A plan with no
//! configured workflow therefore behaves exactly as it does today, and
//! there is no seeded row for an operator to edit into something that
//! silently changes every existing board.

use sea_orm_migration::prelude::*;

/// The workflows migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the three workflow tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS workflows (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     -- NULL scopes the workflow deployment-wide; a value
                     -- scopes it to that plan, which then overrides.
                     plan_pid UUID NULL,
                     name VARCHAR NOT NULL,
                     applies_to VARCHAR NOT NULL
                         CHECK (applies_to IN ('task', 'issue')),
                     is_default BOOLEAN NOT NULL DEFAULT false,
                     deleted_at TIMESTAMPTZ NULL
                 );
                 CREATE INDEX IF NOT EXISTS workflows_plan ON workflows (plan_pid, applies_to);
                 -- At most one plan-scoped workflow per resource kind,
                 -- so `the workflow in force` is never ambiguous.
                 CREATE UNIQUE INDEX IF NOT EXISTS workflows_one_per_plan
                     ON workflows (plan_pid, applies_to)
                     WHERE deleted_at IS NULL AND plan_pid IS NOT NULL;

                 CREATE TABLE IF NOT EXISTS workflow_states (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     workflow_pid UUID NOT NULL,
                     state_key VARCHAR NOT NULL,
                     label VARCHAR NOT NULL,
                     -- The load-bearing column. See the module note.
                     category VARCHAR NOT NULL
                         CHECK (category IN ('todo', 'active', 'waiting', 'done')),
                     wip_limit INTEGER NULL CHECK (wip_limit IS NULL OR wip_limit > 0),
                     is_initial BOOLEAN NOT NULL DEFAULT false,
                     is_terminal BOOLEAN NOT NULL DEFAULT false,
                     position INTEGER NOT NULL DEFAULT 0
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS workflow_states_key
                     ON workflow_states (workflow_pid, state_key);
                 -- Exactly one initial state per workflow: zero leaves
                 -- new work nowhere to land, two make it ambiguous.
                 CREATE UNIQUE INDEX IF NOT EXISTS workflow_states_one_initial
                     ON workflow_states (workflow_pid) WHERE is_initial;

                 CREATE TABLE IF NOT EXISTS workflow_transitions (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     pid UUID NOT NULL UNIQUE,
                     workflow_pid UUID NOT NULL,
                     from_key VARCHAR NOT NULL,
                     to_key VARCHAR NOT NULL
                 );
                 CREATE UNIQUE INDEX IF NOT EXISTS workflow_transitions_pair
                     ON workflow_transitions (workflow_pid, from_key, to_key);",
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
                "DROP TABLE IF EXISTS workflow_transitions;
                 DROP TABLE IF EXISTS workflow_states;
                 DROP TABLE IF EXISTS workflows;",
            )
            .await?;
        Ok(())
    }
}
