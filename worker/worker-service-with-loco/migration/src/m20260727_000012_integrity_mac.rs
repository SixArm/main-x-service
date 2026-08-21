//! Migration: add the keyed-integrity (HMAC) columns.
//!
//! The unkeyed digests are forgeable by anyone who can write SQL, since
//! their pre-image format is published. A MAC keyed from the service
//! environment — never from this database — is not. See the accompanying
//! SQL and `spec/12-compliance.md` §12.4z.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The keyed-integrity migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260727_000012_integrity_mac"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072700000012_integrity_mac/up.sql"
            ))
            .await?;
        Ok(())
    }

    /// Drop the columns (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072700000012_integrity_mac/down.sql"
            ))
            .await?;
        Ok(())
    }
}
