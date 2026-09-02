//! Migration: **multi-action automation rules** (entity spec FR-32,
//! repo `tasks.md` PRO-P20 / T-21). "More than one action per rule,
//! applied in declared order with each action's outcome logged
//! separately" — the two single-value columns `automations.action_kind`
//! / `action_value` cannot express an ordered list at all, so this is a
//! genuine schema change, not an additive one.
//!
//! ## Why a JSONB array, not a child table
//!
//! `actions JSONB NOT NULL DEFAULT '[]'` holds `[{"kind": …, "value":
//! …}, …]`; array order **is** declared order, so no separate
//! `position` column is needed. A child table (mirroring
//! `control_actions`, say) was considered and rejected here: those
//! child rows are independently queried and mutated (closed, converted,
//! …), where an automation's actions are never addressed individually —
//! they are read, validated, and applied as one unit every time the
//! rule is read or fires. A JSONB array is the same shape the single
//! `action_value` column already used, extended by one level rather
//! than restructured.
//!
//! ## Why `automation_runs` gains `action_index`
//!
//! "Each action's outcome logged separately" means one firing of a
//! three-action rule writes **three** run rows, not one — otherwise a
//! partial failure (action 2 of 3 fails) would have nowhere honest to
//! record itself. `action_index` (0-based, matching the `actions` array
//! position) is what lets two rows from the same firing be told apart;
//! `NOT NULL DEFAULT 0` keeps every pre-existing single-action run
//! correctly addressable as "the rule's action 0" with no backfill.

use sea_orm_migration::prelude::*;

/// The automation-multi-action migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Replace `automations.action_kind`/`action_value` with one
    /// ordered `actions` array; add `automation_runs.action_index`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE automations
                     ADD COLUMN actions JSONB NOT NULL DEFAULT '[]'::jsonb;
                 -- Backfill the single existing action into the new
                 -- array shape before the old columns are dropped, so
                 -- no in-flight rule is silently emptied.
                 UPDATE automations
                     SET actions = jsonb_build_array(
                         jsonb_build_object('kind', action_kind, 'value', action_value)
                     );
                 ALTER TABLE automations
                     DROP COLUMN action_kind,
                     DROP COLUMN action_value;
                 -- A rule with zero actions could never do anything;
                 -- refused at write time by validate_actions, and
                 -- enforced here too so a direct insert can't bypass it.
                 ALTER TABLE automations
                     ADD CONSTRAINT automations_actions_not_empty
                     CHECK (jsonb_array_length(actions) > 0);

                 ALTER TABLE automation_runs
                     ADD COLUMN action_index INTEGER NOT NULL DEFAULT 0
                     CHECK (action_index >= 0);
                 -- One logged outcome per (firing, action position):
                 -- a rule with three actions logs three rows sharing
                 -- everything but action_index, never one row silently
                 -- overwritten by the next action's outcome.
                 CREATE INDEX IF NOT EXISTS automation_runs_automation_action
                     ON automation_runs (automation_pid, action_index);",
            )
            .await?;
        Ok(())
    }

    /// Reverse: restore the single-action columns from `actions[0]`
    /// (later elements are lost — the same information `up` cannot
    /// recover either way), drop `action_index`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE automations
                     ADD COLUMN action_kind VARCHAR NOT NULL DEFAULT '',
                     ADD COLUMN action_value JSONB NOT NULL DEFAULT '{}'::jsonb;
                 UPDATE automations
                     SET action_kind = COALESCE(actions -> 0 ->> 'kind', ''),
                         action_value = COALESCE(actions -> 0 -> 'value', '{}'::jsonb);
                 ALTER TABLE automations
                     DROP CONSTRAINT IF EXISTS automations_actions_not_empty,
                     DROP COLUMN actions;

                 DROP INDEX IF EXISTS automation_runs_automation_action;
                 ALTER TABLE automation_runs DROP COLUMN action_index;",
            )
            .await?;
        Ok(())
    }
}
