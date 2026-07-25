//! Migration: create the `audit_logs` table — the HIPAA-style who/what/
//! when trail. Case data is personal data, so every CRUD/merge action
//! writes one row here recording the action, the (optional) actor, and a
//! snapshot of the record at that time.

use loco_rs::schema::{create_table, drop_table, ColType};
use sea_orm_migration::prelude::*;

/// The `audit_logs`-table migration. `DeriveMigrationName` derives the
/// name from the timestamped module path.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `audit_logs` table.
    ///
    /// # Errors
    ///
    /// When the `CREATE TABLE` fails.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "audit_logs",
            &[
                // Internal auto-increment primary key.
                ("id", ColType::PkAuto),
                // The case pid the entry concerns.
                ("entity_pid", ColType::Uuid),
                // created / updated / deleted / merged / merged_into.
                ("action", ColType::String),
                // Optional actor (verified caller `sub` / system); null
                // until a bearer token is presented.
                ("actor", ColType::StringNull),
                // Snapshot of the record at the time of the action.
                ("snapshot", ColType::JsonBinaryNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `audit_logs` table (rollback).
    ///
    /// # Errors
    ///
    /// When the `DROP TABLE` fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "audit_logs").await?;
        Ok(())
    }
}
