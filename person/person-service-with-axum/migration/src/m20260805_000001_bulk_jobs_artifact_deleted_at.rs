//! Migration: add `bulk_jobs.artifact_deleted_at` (SEC-B4 follow-up) —
//! records that the periodic physical-artifact sweep has already
//! removed a job's artifacts from the store, so the same row is never
//! swept twice. Wraps the hand-written SQL under
//! `../../migrations/2026080500000001_bulk_jobs_artifact_deleted_at/{up,down}.sql`
//! via `include_str!`, matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260805_000001_bulk_jobs_artifact_deleted_at"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026080500000001_bulk_jobs_artifact_deleted_at/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026080500000001_bulk_jobs_artifact_deleted_at/down.sql"
            ))
            .await?;
        Ok(())
    }
}
