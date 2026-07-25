//! Migration: create the `cases` table — the registry of governmental
//! case records. The full `case_matcher::Case` payload is stored as JSONB
//! in `data`; `pid` and `title` are denormalised for lookup and listing.

use loco_rs::schema::{create_table, drop_table, ColType};
use sea_orm_migration::prelude::*;

/// The `cases`-table migration. `DeriveMigrationName` derives the name
/// from the module path (the timestamped file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `cases` table.
    ///
    /// # Errors
    ///
    /// When the `CREATE TABLE` fails (e.g. the table already exists).
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "cases",
            &[
                // Internal auto-increment primary key.
                ("id", ColType::PkAuto),
                // Public, externally exposed UUID; unique.
                ("pid", ColType::UuidUniq),
                // Denormalised title, for lookup / listing / ILIKE search.
                ("title", ColType::String),
                // Full `case_matcher::Case` payload as JSON.
                ("data", ColType::JsonBinary),
                // Active flag; cleared on soft-delete.
                ("active", ColType::BooleanWithDefault(true)),
                // Soft-delete timestamp; null while active.
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `cases` table (rollback).
    ///
    /// # Errors
    ///
    /// When the `DROP TABLE` fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "cases").await?;
        Ok(())
    }
}
