//! Migration: add `cases.content_hash` for row-level record integrity.
//!
//! The audit chain (`m20260726_000006_compliance`) proves the **trail**
//! was not rewritten. It says nothing about the case rows themselves: an
//! attacker with SQL access could edit a stored case — its title, its
//! subjects, its agency case number — and, writing no audit row, leave
//! the chain verifying. Until now this service was the only one of the
//! four carrying an audit chain that had no answer to that; person,
//! worker and care-pathway all do.
//!
//! Nullable, and existing rows stay NULL on purpose. Back-filling would
//! compute each hash from the current content, asserting that the current
//! content is authentic — exactly the claim this column exists to test.
//! A back-filled hash would certify whatever an attacker had already
//! changed. Rows are hashed on their next write; until then verification
//! reports them as `unhashed`, which is an honest gap rather than a false
//! clean bill of health.

use sea_orm_migration::prelude::*;

/// The `cases.content_hash` migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Column identifiers for the `cases` table touched here.
#[derive(DeriveIden)]
enum Cases {
    /// The table itself.
    Table,
    /// SHA-256 over the record's content and lifecycle state.
    ContentHash,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Cases::Table)
                .add_column_if_not_exists(ColumnDef::new(Cases::ContentHash).string().null())
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Drop the column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Cases::Table)
                .drop_column(Cases::ContentHash)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
