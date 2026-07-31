//! Migration: the asset library and its declared renditions (CMS-R6–R8).
//!
//! `assets.checksum_sha256` is the content address: the same bytes
//! always land under the same storage key, so re-uploading a file
//! deduplicates the stored object rather than accumulating copies, and
//! an uploader can never choose where their bytes go (CMS-D9).
//!
//! `assets.site_pid` is nullable: an asset may be shared across a
//! deployment rather than belonging to one site.
//!
//! Renditions are *declarations* in v1 — dimensions and a state — with
//! `storage_ref` null until something produces the bytes. Delivery
//! reports only the renditions that exist, so a declared-but-unproduced
//! variant never becomes a broken URL (spec `assets.md`).

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
            "CREATE TABLE IF NOT EXISTS assets (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NULL,
                 kind VARCHAR NOT NULL,
                 mime VARCHAR NOT NULL,
                 byte_size BIGINT NOT NULL,
                 checksum_sha256 VARCHAR NOT NULL,
                 storage_ref VARCHAR NOT NULL,
                 original_filename VARCHAR NULL,
                 title VARCHAR NULL,
                 alt_text VARCHAR NULL,
                 caption VARCHAR NULL,
                 credit VARCHAR NULL,
                 licence VARCHAR NULL,
                 tags JSONB NOT NULL DEFAULT '[]'::jsonb,
                 width INTEGER NULL,
                 height INTEGER NULL,
                 duration_ms INTEGER NULL,
                 uploaded_by_ref VARCHAR NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS renditions (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 asset_pid UUID NOT NULL,
                 key VARCHAR NOT NULL,
                 width INTEGER NULL,
                 height INTEGER NULL,
                 format VARCHAR NOT NULL,
                 storage_ref VARCHAR NULL,
                 state VARCHAR NOT NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        // The dedupe lookup: one live row per (site, checksum) is what
        // `on_duplicate=reuse` returns instead of storing the bytes twice.
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS assets_site_checksum \
             ON assets (site_pid, checksum_sha256) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS assets_site_kind ON assets (site_pid, kind)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS renditions_asset_key_live \
             ON renditions (asset_pid, key) WHERE deleted_at IS NULL",
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
        for table in ["renditions", "assets"] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
