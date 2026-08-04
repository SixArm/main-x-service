//! Migration: create the `suggestion_runs` table — the durable per-pass
//! audit trail for the periodic cross-service `same_identity` suggestion
//! job (spec T-33, design §16 OQ-9(d)). SQL is the source of truth; this
//! wrapper bridges it into the loco `Migrator`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The `suggestion_runs` table migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260804_000001_suggestion_runs"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026080400000001_suggestion_runs/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../migrations/2026080400000001_suggestion_runs/down.sql"
            ))
            .await?;
        Ok(())
    }
}
