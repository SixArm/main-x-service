//! Migration: drop the BLAKE3 companion digests.
//!
//! BLAKE3 is not FIPS/NIST approved, so it cannot be the control of
//! record here. SHA-256 (FIPS 180-4) and SHA-3 (FIPS 202) remain — and
//! are the better structural pair anyway, Merkle-Damgard against sponge.
//! See the accompanying SQL and `spec/12-compliance.md` §12.4z.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The drop-BLAKE3 migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260727_000010_drop_blake3"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Drop the columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("../../migrations/2026072700000010_drop_blake3/up.sql"))
            .await?;
        Ok(())
    }

    /// Recreate them, empty (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("../../migrations/2026072700000010_drop_blake3/down.sql"))
            .await?;
        Ok(())
    }
}
