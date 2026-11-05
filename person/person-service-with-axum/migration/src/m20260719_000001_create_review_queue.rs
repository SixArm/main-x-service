//! Migration: create the `review_queue` table — the stored
//! deduplication review queue (batch-scan candidates + operator
//! decisions). Pairs are stored normalized (`record_id_a <
//! record_id_b`) under a UNIQUE constraint so re-scans upsert in
//! place and decided rows keep their decision. Wraps the hand-written
//! SQL under `../../migrations/2026071900000001_create_review_queue/{up,down}.sql`
//! via `include_str!`, matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260719_000001_create_review_queue"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026071900000001_create_review_queue/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026071900000001_create_review_queue/down.sql"
            ))
            .await?;
        Ok(())
    }
}
