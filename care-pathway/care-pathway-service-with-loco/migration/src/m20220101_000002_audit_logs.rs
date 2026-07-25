//! Migration: create the `audit_logs` table — one row per CRUD/merge
//! action on a care pathway (who / what / when, plus a payload snapshot).

use loco_rs::schema::{ColType, create_table, drop_table};
use sea_orm_migration::prelude::*;

/// The `audit_logs` table migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `audit_logs` table and its columns.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "audit_logs",
            &[
                ("id", ColType::PkAuto),
                // The care-pathway pid the entry concerns.
                ("entity_pid", ColType::Uuid),
                // created / updated / deleted.
                ("action", ColType::String),
                // Optional actor (user id / system).
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
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "audit_logs").await?;
        Ok(())
    }
}
