//! Migration: create the `organizations` table — the service's primary
//! store. The full `Organization` payload lives in the `data` JSONB
//! column; `pid` and `name` are denormalised for lookup and search.

use loco_rs::schema::{ColType, create_table, drop_table};
use sea_orm_migration::prelude::*;

/// The `organizations` table migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `organizations` table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "organizations",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("name", ColType::String),
                // Full `organization_matcher::Organization` payload as JSON.
                ("data", ColType::JsonBinary),
                ("active", ColType::BooleanWithDefault(true)),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `organizations` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "organizations").await?;
        Ok(())
    }
}
