//! Migration: create the `audit_logs` table — one row per CRUD action on
//! an organization (who / what / when + an optional payload snapshot).

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The `audit_logs` table migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `audit_logs` table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "audit_logs",
            &[
                ("id", ColType::PkAuto),
                // The organization pid the entry concerns.
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
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "audit_logs").await?;
        Ok(())
    }
}
