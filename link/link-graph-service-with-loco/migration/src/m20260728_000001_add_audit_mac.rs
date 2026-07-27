//! Migration: add the keyed-integrity column to `audit_log`.
//!
//! An HMAC-SHA256 over each audit row's pre-image, keyed by a secret held
//! in the service environment and never written to this database — so a
//! stolen backup, a replica, or a SQL-injection foothold cannot forge one.
//!
//! Only the audit trail gets one. The `edges` table is a derived
//! read-model rebuilt from the entity event streams, so a MAC there would
//! attest to a projection rather than a source of truth.
//!
//! Nullable and never back-filled: a MAC computed later would certify
//! whatever the row now says, which is the claim it exists to test.

use sea_orm_migration::prelude::*;

/// The audit-MAC migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Columns added to `audit_log`.
#[derive(DeriveIden)]
enum AuditLog {
    /// The table itself.
    Table,
    /// HMAC over this row's pre-image, as `"<scheme>.<key id>:<hex>"`.
    Mac,
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
                .table(AuditLog::Table)
                .add_column_if_not_exists(ColumnDef::new(AuditLog::Mac).string().null())
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
                .table(AuditLog::Table)
                .drop_column(AuditLog::Mac)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
