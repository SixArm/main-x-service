//! Migration: add `sessions.source_ip` (T-14).
//!
//! [`agents/share/auditability.md`](../../../../agents/share/auditability.md)
//! documents family-wide audit rows as tracking `user_id,
//! user_ip_address, user_agent`, and this crate is the one place in the
//! family that issues every session and never recorded where the
//! request came from. Nullable and best-effort, exactly like the
//! existing `user_agent` column: neither is backfillable for rows
//! issued before this migration.

use sea_orm_migration::prelude::*;

/// The `sessions.source_ip` migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Column added to `sessions`.
#[derive(DeriveIden)]
enum Sessions {
    /// The table itself.
    Table,
    /// Best-effort connecting-peer address captured at issuance.
    SourceIp,
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
                .table(Sessions::Table)
                .add_column_if_not_exists(ColumnDef::new(Sessions::SourceIp).string().null())
                .to_owned(),
        )
        .await
    }

    /// Drop the column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Sessions::Table)
                .drop_column(Sessions::SourceIp)
                .to_owned(),
        )
        .await
    }
}
