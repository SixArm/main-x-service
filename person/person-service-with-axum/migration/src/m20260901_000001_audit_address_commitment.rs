//! Migration: add `audit_log.ip_salt` and `audit_log.ip_commitment`, so that a
//! row can lose its caller's address and keep its hash. Wraps the hand-written
//! SQL under `../../migrations/2026090100000001_audit_address_commitment/{up,down}.sql`
//! via `include_str!`, matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260901_000001_audit_address_commitment"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026090100000001_audit_address_commitment/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026090100000001_audit_address_commitment/down.sql"
            ))
            .await?;
        Ok(())
    }
}
