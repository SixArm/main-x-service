//! Migration: **`tasks.flow_type`** — the declared work-item type
//! behind Flow Distribution (entity spec §5.9.5 / FR-31).
//!
//! ## Why declared rather than derived
//!
//! The entity spec first said the type could be *derived* from records
//! already held — feature from a task's `goal_id`, defect from an
//! issue's `kind`. Neither exists: `tasks` has no `goal_id` (objectives
//! link to **plans**), and there is no `issues` table at all (FR-14 is
//! specified and unbuilt — §14.2). A derivation over absent fields
//! would have classified every task `unclassified` while looking like
//! it worked.
//!
//! Declaring the type is also what the Flow Framework itself assumes:
//! it classifies work items by type rather than reconstructing type
//! from structure.
//!
//! ## Why nullable, and why no default
//!
//! `NULL` means **nobody declared one**, which is reported as
//! `unclassified` and counted separately. Defaulting to `'feature'`
//! would silently inflate the one share a reader is most likely to act
//! on, and every existing task would arrive pre-classified as work
//! nobody classified.

use sea_orm_migration::prelude::*;

/// The `flow_type` migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the nullable, CHECK-constrained column.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE tasks ADD COLUMN IF NOT EXISTS flow_type VARCHAR NULL
                     CHECK (flow_type IS NULL OR flow_type IN
                         ('feature', 'defect', 'risk', 'debt'));
                 -- `unclassified` is deliberately NOT a storable value:
                 -- it is the *absence* of a declaration, and giving it a
                 -- spelling would let a row claim to have been
                 -- classified as unclassified.
                 CREATE INDEX IF NOT EXISTS tasks_flow_type ON tasks (flow_type);",
            )
            .await?;
        Ok(())
    }

    /// Drop the column.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE tasks DROP COLUMN IF EXISTS flow_type;")
            .await?;
        Ok(())
    }
}
