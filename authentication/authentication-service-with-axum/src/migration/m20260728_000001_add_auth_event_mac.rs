//! Migration: add the keyed-integrity column to `auth_events`.
//!
//! An HMAC-SHA256 over each row's pre-image, keyed by a secret held in
//! the service environment and never written to this database — so a
//! stolen backup, a replica, or a SQL-injection foothold cannot forge one.
//!
//! This trail records who logged in and who was granted which
//! authorization attributes. An attacker who escalated privilege and
//! could then edit the `attributes_assigned` row would erase the only
//! account of how they did it.
//!
//! Nullable and never back-filled: a MAC computed later would certify
//! whatever the row now says, which is the claim it exists to test.

use sea_orm_migration::prelude::*;

/// The auth-event MAC migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

/// Columns added to `auth_events`.
#[derive(DeriveIden)]
enum AuthEvents {
    /// The table itself.
    Table,
    /// SHA-256 over this row's pre-image.
    Hash,
    /// SHA3-256 over the same pre-image.
    HashSha3,
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
                .table(AuthEvents::Table)
                .add_column_if_not_exists(ColumnDef::new(AuthEvents::Hash).string().null())
                .add_column_if_not_exists(ColumnDef::new(AuthEvents::HashSha3).string().null())
                .add_column_if_not_exists(ColumnDef::new(AuthEvents::Mac).string().null())
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
                .table(AuthEvents::Table)
                .drop_column(AuthEvents::Hash)
                .drop_column(AuthEvents::HashSha3)
                .drop_column(AuthEvents::Mac)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
