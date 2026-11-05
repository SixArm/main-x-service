//! Reshape `sessions` toward the shared-§3 schema
//! (`agents/share/authentication-sessions.md` §3): add the **idle** and
//! **absolute** TTL columns and a `last_seen_at` sliding-window marker,
//! plus the partial index over active sessions. This lets validity be
//! computed from an idle window (slides on use) and an absolute ceiling
//! (never extended) instead of the single boot-time `expires_at`, which
//! the old `Model::is_active` did not even enforce.
//!
//! The existing opaque session id stays the `jid` column (a `sid`-pk
//! rename is a larger, riskier migration and lower value — `jid` already
//! *is* the opaque `sid`). New columns are **nullable** so this migrates
//! an already-populated table without a backfill; new sessions set all
//! three, and `is_active` treats a `NULL` bound as "unbounded" (legacy
//! grace).

use sea_orm_migration::prelude::*;

/// The `sessions` TTL-reshape migration (name from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add `last_seen_at` / `idle_expires_at` / `absolute_expires_at` and
    /// the `sessions_active_user` partial index (active sessions only).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("sessions"))
                .add_column(ColumnDef::new(Alias::new("last_seen_at")).timestamp_with_time_zone())
                .add_column(
                    ColumnDef::new(Alias::new("idle_expires_at")).timestamp_with_time_zone(),
                )
                .add_column(
                    ColumnDef::new(Alias::new("absolute_expires_at")).timestamp_with_time_zone(),
                )
                .to_owned(),
        )
        .await?;
        // Partial index over active sessions — the common lookup filters
        // `revoked_at IS NULL`, so index only those rows.
        m.get_connection()
            .execute_unprepared(
                "CREATE INDEX IF NOT EXISTS sessions_active_user \
                 ON sessions (user_pid) WHERE revoked_at IS NULL",
            )
            .await?;
        Ok(())
    }

    /// Drop the partial index and the three TTL columns (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS sessions_active_user")
            .await?;
        m.alter_table(
            Table::alter()
                .table(Alias::new("sessions"))
                .drop_column(Alias::new("absolute_expires_at"))
                .drop_column(Alias::new("idle_expires_at"))
                .drop_column(Alias::new("last_seen_at"))
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
