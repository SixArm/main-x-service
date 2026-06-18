//! Migration 1: create the `things` aggregate tables.
//!
//! Creates the `things` row table plus its child collection tables
//! (`thing_alternate_names`, `thing_identifiers`, `thing_images`,
//! `thing_same_as`). The actual DDL lives in the sibling `up.sql` / `down.sql`
//! files and is `include_str!`-embedded so the SQL remains the single source
//! of truth; this Rust shim only lets loco's `Migrator` run it.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// Zero-sized migration marker registered with the loco `Migrator`.
pub struct Migration;

impl MigrationName for Migration {
    /// Stable migration name — also the key recorded in `seaql_migrations`.
    fn name(&self) -> &str {
        "m20260608_000001_create_things"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Apply the migration by executing the embedded `up.sql` verbatim.
    ///
    /// # Errors
    ///
    /// Returns [`DbErr`] if the SQL fails to execute (e.g. a table already
    /// exists or the connection is lost).
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // execute_unprepared: the SQL is a multi-statement DDL script, not a
        // single parameterised query.
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026060800000001_create_things/up.sql"
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
                "../../migrations/2026060800000001_create_things/down.sql"
            ))
            .await?;
        Ok(())
    }
}
