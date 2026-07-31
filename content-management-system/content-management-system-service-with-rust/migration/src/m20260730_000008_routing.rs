//! Migration: routes, redirects, menus, and audience rules
//! (CMS-R17–R20).
//!
//! The invariants live in the indexes, not only in the code:
//!
//! - **One current path per (site, locale)** — a partial unique index
//!   over `is_current`, so two live pages cannot claim the same
//!   address. Superseded routes are kept (not deleted) because they are
//!   how a page's address history is answerable.
//! - **One redirect per (site, locale, `from_path`)** — a path cannot
//!   point two ways at once.
//!
//! `redirects.to_path` is nullable: a `410` marker says "this is gone"
//! rather than sending a reader somewhere that is not what they asked
//! for.

use sea_orm_migration::prelude::*;

/// The migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the tables + indexes.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    #[allow(clippy::too_many_lines)] // one literal schema
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS routes (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NOT NULL,
                 locale VARCHAR NOT NULL,
                 path VARCHAR NOT NULL,
                 variant_pid UUID NOT NULL,
                 is_current BOOLEAN NOT NULL DEFAULT TRUE
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS redirects (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NOT NULL,
                 locale VARCHAR NOT NULL,
                 from_path VARCHAR NOT NULL,
                 to_path VARCHAR NULL,
                 status INTEGER NOT NULL,
                 reason VARCHAR NOT NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS menus (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NOT NULL,
                 locale VARCHAR NOT NULL,
                 key VARCHAR NOT NULL,
                 items JSONB NOT NULL DEFAULT '[]'::jsonb,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS audience_rules (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NOT NULL,
                 key VARCHAR NOT NULL,
                 name VARCHAR NOT NULL,
                 predicate JSONB NOT NULL,
                 active BOOLEAN NOT NULL DEFAULT TRUE,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;

        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS routes_current_path \
             ON routes (site_pid, locale, path) WHERE is_current",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS routes_variant ON routes (variant_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS redirects_from \
             ON redirects (site_pid, locale, from_path)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS menus_site_locale_key_live \
             ON menus (site_pid, locale, key) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS audience_rules_site_key_live \
             ON audience_rules (site_pid, key) WHERE deleted_at IS NULL",
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
        for table in ["audience_rules", "menus", "redirects", "routes"] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
