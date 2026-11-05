//! Migration: add `review_queue.provenance` (BLK-2) — how a candidate
//! pair was first surfaced (`operator` scan vs `import` keyless-row
//! duplicate detection). Wraps the hand-written SQL under
//! `../../migrations/2026080200000001_review_queue_provenance/{up,down}.sql`
//! via `include_str!`, matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260802_000001_review_queue_provenance"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026080200000001_review_queue_provenance/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026080200000001_review_queue_provenance/down.sql"
            ))
            .await?;
        Ok(())
    }
}
