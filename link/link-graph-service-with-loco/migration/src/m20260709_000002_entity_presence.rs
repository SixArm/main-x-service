//! Migration: create the `entity_presence` existence-oracle table
//! (spec §10.2). SQL is the source of truth; this wrapper bridges it
//! into the loco `Migrator`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The `entity_presence` table migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260709_000002_entity_presence"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026070900000002_entity_presence/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026070900000002_entity_presence/down.sql"
            ))
            .await?;
        Ok(())
    }
}
