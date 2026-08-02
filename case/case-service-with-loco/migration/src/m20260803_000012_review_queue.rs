//! Migration: create the `review_queue` table — the stored
//! duplicate-review queue, adopted here as part of BLK-5 to route a
//! **keyless** bulk-import row's likely duplicate to an operator
//! (`agents/share/bulk-import-export.md` §6) — mirroring the
//! person/worker/place/thing/organization registries. Pairs are stored
//! normalized (`record_id_a < record_id_b`) under a UNIQUE constraint so
//! re-scans upsert in place and decided rows keep their decision.
//!
//! Unlike organization's original `review_queue` migration, `provenance`
//! is part of the initial table shape rather than a follow-up migration:
//! case has no pre-existing batch-deduplication scan to preserve
//! compatibility with, so there is no reason to add the column in two
//! steps. `provenance` records how a pair was first surfaced (`operator`
//! / `import` / `matcher_suggested` — the cross-service-linking
//! provenance vocabulary) and is never touched by a re-scan upsert.

use sea_orm_migration::prelude::*;

/// The `review_queue` table migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `review_queue` table plus the status index.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS review_queue (
                     id UUID PRIMARY KEY,
                     record_id_a UUID NOT NULL,
                     record_id_b UUID NOT NULL,
                     match_score DOUBLE PRECISION NOT NULL,
                     match_quality VARCHAR NOT NULL,
                     detection_method VARCHAR NOT NULL,
                     score_breakdown JSONB NULL,
                     status VARCHAR NOT NULL DEFAULT 'pending',
                     provenance VARCHAR NOT NULL DEFAULT 'operator',
                     reviewed_by VARCHAR NULL,
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     reviewed_at TIMESTAMPTZ NULL,
                     CONSTRAINT review_queue_pair_unique UNIQUE (record_id_a, record_id_b)
                 );
                 CREATE INDEX IF NOT EXISTS review_queue_status_idx
                     ON review_queue (status);",
            )
            .await?;
        Ok(())
    }

    /// Drop the `review_queue` table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS review_queue;")
            .await?;
        Ok(())
    }
}
