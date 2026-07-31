//! Migration: the translation workflow's per-variant state (CMS-R15)
//! and the per-content-type `unpublish_on_stale` opt-in.
//!
//! The variant already records `translation_of_revision_pid` — **the
//! exact source revision a translation was made from**, which is what
//! makes staleness computable at all. What this adds is the workflow
//! around it: who asked, who is doing it, when it is due, and where it
//! has got to.
//!
//! `content_types.unpublish_on_stale` defaults **false**, because
//! stale-but-published usually beats absent and that judgement belongs
//! to an editor. A safety notice or legal text is where a deployment
//! turns it on (spec `localization.md`).

use sea_orm_migration::prelude::*;

/// The migration.
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the translation columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for column in [
            "translation_status VARCHAR NULL",
            "translation_requested_at TIMESTAMPTZ NULL",
            "translation_requested_by VARCHAR NULL",
            "translation_due_on DATE NULL",
            "translator_ref VARCHAR NULL",
        ] {
            conn.execute_unprepared(&format!(
                "ALTER TABLE entry_variants ADD COLUMN IF NOT EXISTS {column}"
            ))
            .await?;
        }
        conn.execute_unprepared(
            "ALTER TABLE content_types ADD COLUMN IF NOT EXISTS \
             unpublish_on_stale BOOLEAN NOT NULL DEFAULT FALSE",
        )
        .await?;
        // The translator's queue: open requests, oldest first.
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS entry_variants_translation_status \
             ON entry_variants (translation_status) WHERE translation_status IS NOT NULL",
        )
        .await?;
        Ok(())
    }

    /// Drop the columns (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        for column in [
            "translation_status",
            "translation_requested_at",
            "translation_requested_by",
            "translation_due_on",
            "translator_ref",
        ] {
            conn.execute_unprepared(&format!(
                "ALTER TABLE entry_variants DROP COLUMN IF EXISTS {column}"
            ))
            .await?;
        }
        conn.execute_unprepared(
            "ALTER TABLE content_types DROP COLUMN IF EXISTS unpublish_on_stale",
        )
        .await?;
        Ok(())
    }
}
