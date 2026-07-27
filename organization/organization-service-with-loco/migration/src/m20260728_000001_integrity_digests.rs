//! Migration: add the row-level integrity columns.
//!
//! Three values per row, all nullable and all **never back-filled**:
//!
//! - `content_hash` — SHA-256 (FIPS 180-4) over the row's pre-image.
//! - `content_hash_sha3` — SHA3-256 (FIPS 202). Kept for structural
//!   diversity: a sponge construction, unrelated to SHA-256's
//!   Merkle-Damgard chaining, so a cryptanalytic advance against one
//!   design family does not transfer.
//! - `content_mac` / `mac` — HMAC-SHA256 (FIPS 198-1) over the same
//!   pre-image, keyed.
//!
//! The MAC is the one that matters against a deliberate edit. The two
//! digests are **unkeyed** and their pre-image format is published, so
//! anyone who can write SQL defeats them: edit the row, recompute both,
//! update both columns. What they detect is careless or unaware
//! modification — a bug, a manual fix, a restore from the wrong backup.
//! The MAC raises that bar to a key held in the service environment and
//! never written to this database, so a stolen backup, a replica, a
//! SQL-injection foothold, or a DBA without application-server access
//! cannot forge one.
//!
//! Stored as `"<scheme>.<key id>:<hex>"`. The key id is what makes
//! rotation survivable: without it, changing the key would invalidate
//! every historical row at once, which is indistinguishable from mass
//! tampering.
//!
//! Nullable and never back-filled, because a digest computed later from
//! current content would certify whatever that content now is — which is
//! the claim it exists to test. Rows without one are reported as
//! unhashed or `mac_absent`, never as mismatches.

use sea_orm_migration::prelude::*;

/// The integrity-columns migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Columns added to `audit_logs`.
#[derive(DeriveIden)]
enum AuditLogs {
    /// The table itself.
    Table,
    /// HMAC over this row's pre-image, as `"<scheme>.<key id>:<hex>"`.
    Mac,
}

/// Columns added to `organizations`.
#[derive(DeriveIden)]
enum Organizations {
    /// The table itself.
    Table,
    /// SHA-256 over the record's pre-image.
    ContentHash,
    /// SHA3-256 over the same pre-image.
    ContentHashSha3,
    /// HMAC-SHA256 over the same pre-image.
    ContentMac,
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
                .table(Organizations::Table)
                .add_column_if_not_exists(
                    ColumnDef::new(Organizations::ContentHash).string().null(),
                )
                .add_column_if_not_exists(
                    ColumnDef::new(Organizations::ContentHashSha3)
                        .string()
                        .null(),
                )
                .add_column_if_not_exists(ColumnDef::new(Organizations::ContentMac).string().null())
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
                .table(Organizations::Table)
                .drop_column(Organizations::ContentHash)
                .drop_column(Organizations::ContentHashSha3)
                .drop_column(Organizations::ContentMac)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
