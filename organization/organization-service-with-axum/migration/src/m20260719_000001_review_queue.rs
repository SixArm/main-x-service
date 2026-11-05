//! Migration: create the `review_queue` table — the stored
//! batch-deduplication review queue (scan candidates + operator
//! decisions), mirroring the person/worker/place/thing registries.
//! Pairs are stored normalized (`record_id_a < record_id_b`) under a
//! UNIQUE constraint so re-scans upsert in place and decided rows keep
//! their decision. Explicit SQL (not the pluralizing `create_table`
//! helper), like the `event_outbox` migration.

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
