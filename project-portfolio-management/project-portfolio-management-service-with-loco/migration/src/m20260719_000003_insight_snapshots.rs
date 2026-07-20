//! Migration: create `insight_snapshots` — point-in-time estate
//! captures (portfolio RAG counts, open exposure, per-currency money
//! totals) behind the board/CRO trend views. Snapshots are taken
//! explicitly (`POST /api/board/snapshots`) or by the optional
//! env-gated ticker; trends read the stored series — no invented
//! history.

use sea_orm_migration::prelude::*;

/// The `insight_snapshots` migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table + the kind/taken_at index.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS insight_snapshots (
                     id SERIAL PRIMARY KEY,
                     taken_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     kind VARCHAR NOT NULL,
                     body JSONB NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS insight_snapshots_kind_taken
                     ON insight_snapshots (kind, taken_at);",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS insight_snapshots;")
            .await?;
        Ok(())
    }
}
