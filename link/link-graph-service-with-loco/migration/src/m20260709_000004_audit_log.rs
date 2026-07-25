//! Migration: create the `audit_log` governance table (spec §10.4). SQL
//! is the source of truth; this wrapper bridges it into the loco
//! `Migrator`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The `audit_log` table migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260709_000004_audit_log"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026070900000004_audit_log/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026070900000004_audit_log/down.sql"
            ))
            .await?;
        Ok(())
    }
}
