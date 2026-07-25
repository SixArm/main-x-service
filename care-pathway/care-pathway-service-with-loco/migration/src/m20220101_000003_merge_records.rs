//! Migration: create the `merge_records` table — one row per record-merge
//! (which duplicate folded into which survivor, by whom, with a snapshot
//! of the transferred payload).

use loco_rs::schema::{ColType, create_table, drop_table};
use sea_orm_migration::prelude::*;

/// The `merge_records` table migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `merge_records` table and its columns.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "merge_records",
            &[
                ("id", ColType::PkAuto),
                // The surviving (main) pathway pid.
                ("main_pid", ColType::Uuid),
                // The merged-away (duplicate) pathway pid, now soft-deleted.
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
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "merge_records").await?;
        Ok(())
    }
}
