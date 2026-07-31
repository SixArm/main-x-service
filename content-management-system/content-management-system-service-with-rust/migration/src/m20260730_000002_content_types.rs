//! Migration: content types (CMS-R2) — the operator-declared field
//! schemas that make "what an Article is" data rather than a code
//! change.
//!
//! `fields` is JSONB by decision (CMS-D2): its shape is declared at
//! runtime by an operator, so it cannot be a set of columns. Validation
//! lives in the pure core (`rules::schema`), and `schema_version`
//! records which declaration a stored revision was written under —
//! existing content keeps validating against its own version until it
//! is re-saved (CMS-R2).

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
            "CREATE TABLE IF NOT EXISTS content_types (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NOT NULL,
                 key VARCHAR NOT NULL,
                 name VARCHAR NOT NULL,
                 description VARCHAR NULL,
                 fields JSONB NOT NULL,
                 routable BOOLEAN NOT NULL DEFAULT TRUE,
                 template_key VARCHAR NULL,
                 schema_version INTEGER NOT NULL DEFAULT 1,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS content_types_site_key_live \
             ON content_types (site_pid, key) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS content_types_site ON content_types (site_pid)",
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
            .execute_unprepared("DROP TABLE IF EXISTS content_types")
            .await?;
        Ok(())
    }
}
