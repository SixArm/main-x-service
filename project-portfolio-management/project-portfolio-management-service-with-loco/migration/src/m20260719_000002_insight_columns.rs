//! Migration: the moderate-fit executive-insight columns —
//! stage-gated funding tranches on `budget_lines` (`gate` +
//! `released_at`), the risk `category` (technical-debt register), and
//! the milestone completion timestamp `done_at` (delivery-flow
//! metrics). All nullable, so existing rows keep their behaviour:
//! an ungated line is never held, an uncategorised risk reads as
//! `delivery`, and a milestone completed before this migration counts
//! but is not timed.

use sea_orm_migration::prelude::*;

/// The insight-columns migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the four nullable columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE budget_lines ADD COLUMN IF NOT EXISTS gate VARCHAR NULL;
                 ALTER TABLE budget_lines ADD COLUMN IF NOT EXISTS released_at TIMESTAMPTZ NULL;
                 ALTER TABLE risks ADD COLUMN IF NOT EXISTS category VARCHAR NULL;
                 ALTER TABLE milestones ADD COLUMN IF NOT EXISTS done_at TIMESTAMPTZ NULL;",
            )
            .await?;
        Ok(())
    }

    /// Drop the four columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE budget_lines DROP COLUMN IF EXISTS gate;
                 ALTER TABLE budget_lines DROP COLUMN IF EXISTS released_at;
                 ALTER TABLE risks DROP COLUMN IF EXISTS category;
                 ALTER TABLE milestones DROP COLUMN IF EXISTS done_at;",
            )
            .await?;
        Ok(())
    }
}
