//! Migration 2: create the `audit_log` and `thing_merge_records` tables.
//!
//! Adds the HIPAA-style audit trail table and the merge-history table. As with
//! migration 1, the DDL lives in the sibling `up.sql` / `down.sql` files and is
//! `include_str!`-embedded; this Rust shim only lets loco's `Migrator` run it.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// Zero-sized migration marker registered with the loco `Migrator`.
pub struct Migration;

impl MigrationName for Migration {
    /// Stable migration name — also the key recorded in `seaql_migrations`.
    fn name(&self) -> &str {
        "m20260608_000002_create_audit_and_merge"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Apply the migration by executing the embedded `up.sql` verbatim.
    ///
    /// # Errors
    ///
    /// Returns [`DbErr`] if the SQL fails to execute.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026060800000002_create_audit_and_merge/up.sql"
            ))
            .await?;
        Ok(())
    }

    /// Revert the migration by executing the embedded `down.sql` verbatim.
    ///
    /// # Errors
    ///
    /// Returns [`DbErr`] if the rollback SQL fails to execute.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026060800000002_create_audit_and_merge/down.sql"
            ))
            .await?;
        Ok(())
    }
}
