//! Migration: drop the database-side audit triggers.
//!
//! They appended **unchained** rows to `audit_log`, which made the
//! tamper-evident chain partial while adding nothing the application does
//! not already record with better provenance. The full reasoning is in
//! `../../migrations/2026072600000003_drop_audit_triggers/up.sql`.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// The drop-audit-triggers migration.
pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260726_000003_drop_audit_triggers"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Drop the triggers and their functions.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072600000003_drop_audit_triggers/up.sql"
            ))
            .await?;
        Ok(())
    }

    /// Recreate them (rollback). See the SQL for why you probably should
    /// not.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072600000003_drop_audit_triggers/down.sql"
            ))
            .await?;
        Ok(())
    }
}
