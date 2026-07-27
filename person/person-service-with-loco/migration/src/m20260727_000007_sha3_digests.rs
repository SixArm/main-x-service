//! Migration: add the SHA-3 companion digests.
//!
//! Third algorithm alongside SHA-256 and BLAKE3; see the accompanying SQL
//! and `spec/12-compliance.md` §12.4z for why three.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The SHA-3-digest migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260727_000007_sha3_digests"
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
            .execute_unprepared(include_str!("../../migrations/2026072700000007_sha3_digests/up.sql"))
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
            .execute_unprepared(include_str!("../../migrations/2026072700000007_sha3_digests/down.sql"))
            .await?;
        Ok(())
    }
}
