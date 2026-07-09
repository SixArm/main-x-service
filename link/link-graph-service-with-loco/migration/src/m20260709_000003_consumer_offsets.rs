//! Migration: create the `consumer_offsets` table — per-topic bus
//! position + the freshness watermark backing `as_of` (spec §10.3).
//! SQL is the source of truth; this wrapper bridges it into the loco
//! `Migrator`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The `consumer_offsets` table migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260709_000003_consumer_offsets"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026070900000003_consumer_offsets/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026070900000003_consumer_offsets/down.sql"
            ))
            .await?;
        Ok(())
    }
}
