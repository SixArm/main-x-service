//! Migration: add `care_pathways.content_hash` — the row-level integrity
//! digest.
//!
//! The audit chain proves the *trail* was not rewritten; this closes the
//! complementary gap by letting an out-of-band edit to an entity row be
//! detected. See [`crate`]-level docs and
//! `src/compliance/record_integrity.rs`.
//!
//! Nullable, so rows written before this migration stay valid — verification
//! reports them as `unhashed` rather than as mismatches, and they are
//! rehashed on their next write.

use sea_orm_migration::prelude::*;

/// The record-integrity migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Column identifiers for the `care_pathways` table touched here.
#[derive(DeriveIden)]
enum CarePathways {
    /// The table itself.
    Table,
    /// SHA-256 over the row's content and lifecycle state.
    ContentHash,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the `content_hash` column.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(CarePathways::Table)
                .add_column(ColumnDef::new(CarePathways::ContentHash).string().null())
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Drop the `content_hash` column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(CarePathways::Table)
                .drop_column(CarePathways::ContentHash)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
