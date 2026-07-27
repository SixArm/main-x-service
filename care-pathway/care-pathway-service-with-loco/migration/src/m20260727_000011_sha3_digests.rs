//! Migration: add the SHA-3 companion digests.
//!
//! Third algorithm alongside SHA-256 and BLAKE3, over the same pre-image.
//! The point is **structural diversity**: SHA-256 is Merkle-Damgard,
//! BLAKE3 is an ARX tree, and SHA-3 is a sponge, so a cryptanalytic
//! advance against one design family does not transfer to the others.
//! SHA-3 is also FIPS 202, so it carries the same standards weight as
//! SHA-256 while being structurally independent of it — see
//! `spec/12-compliance.md` §12.4z.
//!
//! Nullable, never back-filled, for the same reason as the others: a
//! digest computed from current content certifies whatever that content
//! now is, which is the claim these columns exist to test.

use sea_orm_migration::prelude::*;

/// The SHA-3-digest migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Columns added to `audit_logs`.
#[derive(DeriveIden)]
enum AuditLogs {
    /// The table itself.
    Table,
    /// SHA-3 digest of the preceding chain row.
    PrevHashSha3,
    /// This row's SHA-3 digest.
    HashSha3,
}

/// Columns added to `care_pathways`.
#[derive(DeriveIden)]
enum CarePathways {
    /// The table itself.
    Table,
    /// SHA-3 digest over the record's content and lifecycle state.
    ContentHashSha3,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(AuditLogs::Table)
                .add_column_if_not_exists(ColumnDef::new(AuditLogs::PrevHashSha3).string().null())
                .add_column_if_not_exists(ColumnDef::new(AuditLogs::HashSha3).string().null())
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(CarePathways::Table)
                .add_column_if_not_exists(
                    ColumnDef::new(CarePathways::ContentHashSha3)
                        .string()
                        .null(),
                )
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Drop the columns (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(AuditLogs::Table)
                .drop_column(AuditLogs::PrevHashSha3)
                .drop_column(AuditLogs::HashSha3)
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(CarePathways::Table)
                .drop_column(CarePathways::ContentHashSha3)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
