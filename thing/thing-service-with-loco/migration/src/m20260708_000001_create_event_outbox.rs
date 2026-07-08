//! Migration 3: create the `event_outbox` table.
//!
//! The transactional-outbox hand-off buffer for the durable event bus
//! (Phase 2; `agents/share/event-bus.md` §3). As with the other
//! migrations, the DDL lives in the sibling `up.sql` / `down.sql` files
//! and is `include_str!`-embedded; this Rust shim only lets loco's
//! `Migrator` run it.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

/// Zero-sized migration marker registered with the loco `Migrator`.
pub struct Migration;

impl MigrationName for Migration {
    /// Stable migration name — also the key recorded in `seaql_migrations`.
    fn name(&self) -> &str {
        "m20260708_000001_create_event_outbox"
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
                "../../migrations/2026070800000001_create_event_outbox/up.sql"
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
                "../../migrations/2026070800000001_create_event_outbox/down.sql"
            ))
            .await?;
        Ok(())
    }
}
