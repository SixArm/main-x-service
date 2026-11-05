//! Migration: add the BLAKE3 companion digests.
//!
//! Two integrity digests over one pre-image — SHA-256 for conservatism
//! and auditor familiarity, BLAKE3 for speed and algorithm agility. The
//! full reasoning, including why the second digest cannot be added
//! retroactively, is in the accompanying SQL and in
//! `spec/12-compliance.md` §12.4z.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The BLAKE3-digest migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260727_000005_blake3_digests"
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
                "../../migrations/2026072700000005_blake3_digests/up.sql"
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
                "../../migrations/2026072700000005_blake3_digests/down.sql"
            ))
            .await?;
        Ok(())
    }
}
