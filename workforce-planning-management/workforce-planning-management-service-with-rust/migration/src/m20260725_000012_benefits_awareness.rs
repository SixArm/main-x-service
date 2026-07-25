//! Migration: benefits awareness (WPM-R26) — generalise the wellbeing
//! entitlement rules with a closed `kind` (`health | benefit`) and an
//! optional link to a benefit plan, so a `benefit` rule can signpost
//! enrolment. The predicate vocabulary is untouched (WPM-D17), and
//! enrolment state is never stored on the rule (WPM-D18).

use sea_orm_migration::prelude::*;

/// The benefits-awareness migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the two columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE wellbeing_entitlements
                     ADD COLUMN IF NOT EXISTS kind VARCHAR NOT NULL DEFAULT 'health',
                     ADD COLUMN IF NOT EXISTS benefit_plan_pid UUID NULL;",
            )
            .await?;
        Ok(())
    }

    /// Drop the two columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE wellbeing_entitlements
                     DROP COLUMN IF EXISTS benefit_plan_pid,
                     DROP COLUMN IF EXISTS kind;",
            )
            .await?;
        Ok(())
    }
}
