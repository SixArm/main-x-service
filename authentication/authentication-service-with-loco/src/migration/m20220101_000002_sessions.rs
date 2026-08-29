//! Create the `sessions` table — one row per server-side cookie session,
//! keyed by `jid` (the session's opaque id, `UNIQUE`, stamped into the
//! short-lived PASETO access token's `sid` claim). This is what makes a
//! session *locally* revocable: signout stamps `revoked_at`, and `/me`
//! rejects a revoked session even though a cached token's PASETO
//! signature still verifies. `expires_at` lets stale rows be reaped.

use loco_rs::schema::{ColType, create_table, drop_table};
use sea_orm_migration::prelude::*;

/// The `sessions`-table migration (name derived from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `sessions` table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "sessions",
            &[
                ("id", ColType::PkAuto),
                ("jid", ColType::StringUniq),
                ("user_pid", ColType::Uuid),
                ("expires_at", ColType::TimestampWithTimeZone),
                ("revoked_at", ColType::TimestampWithTimeZoneNull),
                ("user_agent", ColType::StringNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `sessions` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "sessions").await?;
        Ok(())
    }
}
