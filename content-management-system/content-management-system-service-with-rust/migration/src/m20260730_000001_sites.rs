//! Migration: sites + templates (CMS-R1) — the delivery namespace and
//! the declared presentation contracts.
//!
//! Explicit SQL rather than the loco `create_table` helper (family
//! lesson: the helper pluralizes table names).
//!
//! Two shapes worth noting:
//!
//! - A site's `locales`, `fallback_chains`, and `strict_locales` are
//!   JSONB because they are operator-declared lists/maps whose shape
//!   is validated in the pure core (`rules::locale`), not by DDL.
//! - `templates.regions` is JSONB for the same reason: a region
//!   contract is data a channel reads, never markup this service
//!   renders (CMS-D6).
//!
//! Uniqueness is enforced by **partial** unique indexes over live
//! (not soft-deleted) rows, so a deleted key can be reused.

use sea_orm_migration::prelude::*;

/// The migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables + lookup indexes.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS sites (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 key VARCHAR NOT NULL,
                 name VARCHAR NOT NULL,
                 owner_ref VARCHAR NULL,
                 default_locale VARCHAR NOT NULL,
                 locales JSONB NOT NULL,
                 fallback_chains JSONB NOT NULL,
                 strict_locales JSONB NOT NULL,
                 visibility VARCHAR NOT NULL,
                 base_url VARCHAR NULL,
                 robots_default VARCHAR NOT NULL,
                 require_distinct_approver BOOLEAN NOT NULL DEFAULT TRUE,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS templates (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NOT NULL,
                 key VARCHAR NOT NULL,
                 name VARCHAR NOT NULL,
                 regions JSONB NOT NULL,
                 applies_to_type_keys JSONB NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        // A site key is the delivery namespace's public handle: unique
        // among live sites.
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS sites_key_live \
             ON sites (key) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS templates_site_key_live \
             ON templates (site_pid, key) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS templates_site ON templates (site_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for table in ["templates", "sites"] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
