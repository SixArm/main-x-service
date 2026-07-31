//! Migration: preview tokens (CMS-R22) — the one way unpublished
//! content leaves this service without an editor's credential.
//!
//! Three columns carry the security properties:
//!
//! - **`token_hash`, never the token.** A stolen database must not
//!   yield working preview links, and an operator reading the table
//!   must not be able to impersonate a share. The raw token is returned
//!   exactly once, at issue.
//! - **`revision_pid`**, not just the variant: a token is scoped to the
//!   *specific* revision it was minted for. A link that follows
//!   "whatever is latest" is how embargoed content leaks after the
//!   share was forgotten.
//! - **`expires_at` and `revoked_at`**, so a share is short-lived by
//!   default and can be withdrawn immediately.
//!
//! `used_count` and `last_used_at` exist because a preview share is a
//! disclosure: knowing it was opened forty times after the embargo
//! lifted is the sort of thing an incident review asks for.

use sea_orm_migration::prelude::*;

/// The migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table + lookup indexes.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS preview_tokens (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 token_hash VARCHAR NOT NULL,
                 site_pid UUID NOT NULL,
                 variant_pid UUID NOT NULL,
                 revision_pid UUID NOT NULL,
                 issued_by VARCHAR NULL,
                 expires_at TIMESTAMPTZ NOT NULL,
                 revoked_at TIMESTAMPTZ NULL,
                 used_count INTEGER NOT NULL DEFAULT 0,
                 last_used_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        // The lookup on use: by hash, never by anything guessable.
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS preview_tokens_hash \
             ON preview_tokens (token_hash)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS preview_tokens_variant \
             ON preview_tokens (variant_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS preview_tokens")
            .await?;
        Ok(())
    }
}
