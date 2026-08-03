//! Migration: create the `processed_events` table — bus-consumer
//! idempotency under at-least-once delivery (spec §10.3; BUS-2). SQL is
//! the source of truth; this wrapper bridges it into the loco `Migrator`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The `processed_events` table migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260803_000001_processed_events"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026080300000001_processed_events/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026080300000001_processed_events/down.sql"
            ))
            .await?;
        Ok(())
    }
}
