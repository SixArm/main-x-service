//! Migration: create the `merge_records` table — one row per record
//! merge, capturing the survivor, the merged-away duplicate, an optional
//! reason/actor, and a snapshot of the duplicate's transferred payload.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The `merge_records` table migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `merge_records` table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "merge_records",
            &[
                ("id", ColType::PkAuto),
                // The surviving (main) organization pid.
                ("main_pid", ColType::Uuid),
                // The merged-away (duplicate) pid, now soft-deleted.
                ("duplicate_pid", ColType::Uuid),
                // Optional operator-supplied reason.
                ("reason", ColType::StringNull),
                // Optional actor (user id / system) — null until JWT auth.
                ("actor", ColType::StringNull),
                // Snapshot of the duplicate's payload at merge time.
                ("transferred", ColType::JsonBinaryNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `merge_records` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "merge_records").await?;
        Ok(())
    }
}
