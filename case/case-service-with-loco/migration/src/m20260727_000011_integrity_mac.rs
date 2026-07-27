//! Migration: add the keyed-integrity (HMAC) columns.
//!
//! The SHA-256 and SHA-3 digests are **unkeyed**, and their pre-image
//! format is published in `spec/12-compliance.md` §12.4z. Anyone who can
//! write SQL can therefore defeat them: edit the row, recompute both
//! digests, update both columns. What they actually detect is careless or
//! unaware modification.
//!
//! An HMAC over the same pre-image raises that bar to a key held in the
//! service environment and **never written to this database** — so a
//! stolen backup, a replica, a SQL-injection foothold, or a DBA without
//! application-server access cannot forge one.
//!
//! The column stores `"<key id>:<hex>"`. The key id is what makes
//! rotation survivable: without it, changing the key would invalidate
//! every historical row at once, which is indistinguishable from mass
//! tampering.
//!
//! Nullable, and never back-filled — a MAC computed later from current
//! content would certify whatever that content now is, which is the claim
//! it exists to test. Rows without one are reported `mac_absent`, never
//! as mismatches.

use sea_orm_migration::prelude::*;

/// The keyed-integrity migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Columns added to `audit_logs`.
#[derive(DeriveIden)]
enum AuditLogs {
    /// The table itself.
    Table,
    /// HMAC over this row's pre-image, as `"<key id>:<hex>"`.
    Mac,
}

/// Columns added to `cases`.
#[derive(DeriveIden)]
enum Cases {
    /// The table itself.
    ContentMac,
    /// The table itself.
    Table,
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
                .add_column_if_not_exists(ColumnDef::new(AuditLogs::Mac).string().null())
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(Cases::Table)
                .add_column_if_not_exists(ColumnDef::new(Cases::ContentMac).string().null())
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
                .drop_column(AuditLogs::Mac)
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(Cases::Table)
                .drop_column(Cases::ContentMac)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
