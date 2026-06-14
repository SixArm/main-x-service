//! Migration: create the `merge_records` table — the record-merge
//! history. One row per merge: which duplicate folded into which
//! survivor, by whom, why, and a snapshot of the transferred payload.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The `merge_records`-table migration. `DeriveMigrationName` derives the
/// name from the timestamped module path.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `merge_records` table.
    ///
    /// # Errors
    ///
    /// When the `CREATE TABLE` fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "merge_records",
            &[
                // Internal auto-increment primary key.
                ("id", ColType::PkAuto),
                // The surviving (main) case pid.
                ("main_pid", ColType::Uuid),
                // The merged-away (duplicate) case pid, now soft-deleted.
                ("duplicate_pid", ColType::Uuid),
                // Optional operator-supplied reason.
                ("reason", ColType::StringNull),
                // Optional actor (verified caller `sub` / system) — null
                // until a bearer token is presented.
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
    /// When the `DROP TABLE` fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "merge_records").await?;
        Ok(())
    }
}
