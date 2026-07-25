//! Migration: create the `worker_assessments` table — workforce
//! assessments (aptitude / personality / psychometric / selection)
//! recorded against a worker, with the per-scale outcomes in a
//! `results` JSONB array. Wraps the hand-written SQL under
//! `../../migrations/2026072300000001_create_worker_assessments/{up,down}.sql`
//! via `include_str!`, matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260723_000001_create_worker_assessments"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072300000001_create_worker_assessments/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072300000001_create_worker_assessments/down.sql"
            ))
            .await?;
        Ok(())
    }
}
