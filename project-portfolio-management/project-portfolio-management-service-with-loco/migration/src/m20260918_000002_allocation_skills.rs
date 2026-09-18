//! Migration: **`allocations.skills_required`** — the short tags an
//! allocation declares it needs (T-28c). A JSON array of short
//! strings, matching the family's `tags`/`must_include` convention
//! (`m20220101_000007_strategy.rs`) rather than a Postgres native
//! array.
//!
//! ## Why no skill *data* lands here
//!
//! Only the **requirement** tags are stored. Whether the assigned
//! person actually holds a tag is resolved live, on read, against the
//! worker service by `EntityRef` (`src/workers_client.rs`) — never
//! copied into this row. People stay references (family doctrine); a
//! stored "has skill X" flag would drift the moment the worker
//! service's own record changed.

use sea_orm_migration::prelude::*;

/// The `allocation_skills` migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the `NOT NULL DEFAULT '[]'` column.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE allocations
                     ADD COLUMN IF NOT EXISTS skills_required JSONB NOT NULL DEFAULT '[]';",
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
            .execute_unprepared("ALTER TABLE allocations DROP COLUMN IF EXISTS skills_required;")
            .await?;
        Ok(())
    }
}
