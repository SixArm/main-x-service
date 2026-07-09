//! Migration: create the `edges` read-model table + its three indexes
//! (`from_ref`, `to_ref`, `status`). SQL is the source of truth (spec
//! §10.1); this wrapper bridges it into the loco `Migrator`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The `edges` table migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260709_000001_edges"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!("../migrations/2026070900000001_edges/up.sql"))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026070900000001_edges/down.sql"
            ))
            .await?;
        Ok(())
    }
}
