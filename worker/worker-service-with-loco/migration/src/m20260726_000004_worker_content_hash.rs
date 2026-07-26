//! Migration: add `workers.content_hash` for row-level record integrity.
//!
//! Existing rows stay NULL deliberately — see the SQL for why a back-fill
//! would certify exactly what this column exists to test.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The `workers.content_hash` migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260726_000004_worker_content_hash"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the column.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072600000004_worker_content_hash/up.sql"
            ))
            .await?;
        Ok(())
    }

    /// Drop the column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072600000004_worker_content_hash/down.sql"
            ))
            .await?;
        Ok(())
    }
}
