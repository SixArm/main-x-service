//! Migration: entries, per-locale variants, append-only revisions, and
//! the extracted reference index (CMS-R3, CMS-R5).
//!
//! Three shapes carry the load-bearing invariants (CMS-D2 — these are
//! normalized and constraint-backed precisely because they are *not*
//! operator-defined):
//!
//! - **`entry_variants`** is the unit of workflow: one row per (entry,
//!   locale), each with its own status, its own `current_revision_pid`,
//!   and its own `published_revision_pid`. Publishing points at a
//!   revision, so "saved" and "live" are different columns (CMS-D3).
//! - **`revisions`** is **append-only**: no `deleted_at`, and
//!   `UNIQUE (variant_pid, number)` so the monotonic chain allocated
//!   under the variant row lock can have no gap and no duplicate
//!   (CMS-D15). A restore writes a *new* row recording
//!   `restored_from_pid`.
//! - **`content_references`** is the extracted edge index that makes
//!   "where used" a lookup and delete-refusal enforceable (CMS-D8).
//!   Named `content_references` rather than `references` because the
//!   latter is a reserved SQL keyword that would need quoting at every
//!   use — a footgun for the first person to write raw SQL here.
//!
//! `entries.type_schema_version` records which content-type declaration
//! the entry was created under; a revision inherits it, which is what
//! makes the `needs_migration` reporting (CMS-R21) possible without
//! re-validating a whole corpus on every schema edit.

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
    #[allow(clippy::too_many_lines)] // one literal schema
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS entries (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NOT NULL,
                 content_type_key VARCHAR NOT NULL,
                 type_schema_version INTEGER NOT NULL,
                 key VARCHAR NOT NULL,
                 source_locale VARCHAR NOT NULL,
                 owner_ref VARCHAR NULL,
                 archived_at TIMESTAMPTZ NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS entry_variants (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 entry_pid UUID NOT NULL,
                 locale VARCHAR NOT NULL,
                 status VARCHAR NOT NULL,
                 current_revision_pid UUID NULL,
                 published_revision_pid UUID NULL,
                 translation_of_revision_pid UUID NULL,
                 reviewer_ref VARCHAR NULL,
                 scheduled_publish_at TIMESTAMPTZ NULL,
                 scheduled_unpublish_at TIMESTAMPTZ NULL,
                 locked_by_ref VARCHAR NULL,
                 locked_until TIMESTAMPTZ NULL,
                 published_at TIMESTAMPTZ NULL,
                 first_published_at TIMESTAMPTZ NULL,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        // Append-only: deliberately no `deleted_at`. History that can be
        // deleted is not history (CMS-D3); erasure redacts a body and
        // keeps the row.
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS revisions (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 variant_pid UUID NOT NULL,
                 number INTEGER NOT NULL,
                 title VARCHAR NOT NULL,
                 blocks JSONB NOT NULL,
                 fields JSONB NOT NULL,
                 seo JSONB NOT NULL,
                 type_schema_version INTEGER NOT NULL,
                 author_ref VARCHAR NULL,
                 note VARCHAR NULL,
                 restored_from_pid UUID NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS content_references (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 from_revision_pid UUID NOT NULL,
                 from_variant_pid UUID NOT NULL,
                 kind VARCHAR NOT NULL,
                 to_entry_pid UUID NULL,
                 to_asset_pid UUID NULL,
                 to_entity_ref VARCHAR NULL,
                 field_key VARCHAR NOT NULL
             )",
        )
        .await?;

        // One entry key per site, one variant per locale — both over
        // live rows only, so a soft-deleted key can be reused.
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS entries_site_key_live \
             ON entries (site_pid, key) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS entries_site_type ON entries (site_pid, content_type_key)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS entry_variants_entry_locale_live \
             ON entry_variants (entry_pid, locale) WHERE deleted_at IS NULL",
        )
        .await?;
        // The revision chain: monotonic per variant, no gaps, no
        // duplicates. The unique index is what makes the allocation
        // under `SELECT … FOR UPDATE` provably safe rather than
        // merely usually safe.
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS revisions_variant_number \
             ON revisions (variant_pid, number)",
        )
        .await?;
        // "Where used", in both directions.
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS content_references_from \
             ON content_references (from_revision_pid)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS content_references_to_entry \
             ON content_references (to_entry_pid) WHERE to_entry_pid IS NOT NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS content_references_to_asset \
             ON content_references (to_asset_pid) WHERE to_asset_pid IS NOT NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS content_references_to_entity \
             ON content_references (to_entity_ref) WHERE to_entity_ref IS NOT NULL",
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
        for table in [
            "content_references",
            "revisions",
            "entry_variants",
            "entries",
        ] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
