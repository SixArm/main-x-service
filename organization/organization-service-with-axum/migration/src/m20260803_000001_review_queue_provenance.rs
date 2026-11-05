//! Migration: add `review_queue.provenance` (BLK-5) — how a candidate
//! pair was first surfaced (`operator` batch scan, `POST
//! /organizations/deduplicate`, vs `import` bulk-import keyless-row
//! duplicate detection). Matches the cross-service-linking provenance
//! vocabulary (`operator` | `import` | `matcher_suggested`) and mirrors
//! the person service's `m20260802_000001_review_queue_provenance`
//! migration. Existing rows predate this column and were all
//! operator-triggered scans, hence the backfill default.

use sea_orm_migration::prelude::*;

/// The `review_queue.provenance` migration (name derived from the file).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the `provenance` column, defaulting existing rows to
    /// `'operator'`.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE review_queue \
                 ADD COLUMN IF NOT EXISTS provenance VARCHAR NOT NULL DEFAULT 'operator';",
            )
            .await?;
        Ok(())
    }

    /// Drop the `provenance` column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE review_queue DROP COLUMN IF EXISTS provenance;")
            .await?;
        Ok(())
    }
}
