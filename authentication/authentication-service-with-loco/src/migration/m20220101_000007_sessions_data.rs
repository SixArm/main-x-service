//! Add `sessions.data` — the shared-design session payload column
//! (`agents/share/authentication-sessions.md` §3: "data JSONB — roles,
//! scopes, MFA state, …"). First landed use: session establishment copies
//! the user's ABAC attributes into `data.attrs` so `POST /api/auth/token`
//! mints the `attrs` claim from the session alone (shared
//! `authorization-attributes.md` §6). The rest of the shared-§3 reshape
//! (`sid` pk, `last_seen_at`, idle/absolute TTLs) stays a §13 follow-up.

use sea_orm_migration::prelude::*;

/// The `sessions.data`-column migration (name from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the `sessions.data` column (`JSONB NOT NULL DEFAULT '{}'`).
    /// Written with sea-query directly (not the loco `add_column` helper)
    /// because the helper cannot express the non-null-with-default shape
    /// an already-populated table needs.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("sessions"))
                .add_column(
                    ColumnDef::new(Alias::new("data"))
                        .json_binary()
                        .not_null()
                        .default("{}"),
                )
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Remove the `sessions.data` column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Alias::new("sessions"))
                .drop_column(Alias::new("data"))
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}
