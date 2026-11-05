//! Migration: create the `event_outbox` table — the transactional-outbox
//! hand-off buffer for the durable event bus (Phase 2;
//! `agents/share/event-bus.md` §3). One row is written **in the same
//! transaction** as the entity mutation, so a committed change always has
//! its event and vice versa; a relay worker (Phase 3) later drains
//! unpublished rows to Fluvio and stamps `published_at`.
//!
//! Written as explicit SQL rather than the loco `create_table` helper:
//! the helper **pluralizes** table names (`cruet::to_plural`, so
//! `event_outbox` → `event_outboxes`), which broke a fresh
//! `db migrate` — the index DDL below and the `SeaORM` entity
//! (`table_name = "event_outbox"`) both name the exact table. Safe to
//! rewrite in place: the old helper call could never apply (its own
//! index DDL failed and rolled the migration back), so no database has
//! the pluralized table. Fixed family-wide 2026-07-18.

use sea_orm_migration::prelude::*;

/// The `event_outbox` table migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `event_outbox` table plus the dedup unique index on
    /// `event_id` and the partial index over unpublished rows (the relay
    /// poll target).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS event_outbox (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 -- Auto pk doubles as the global relay order (ORDER BY id).
                 id SERIAL PRIMARY KEY,
                 -- Envelope id — consumer dedup key (unique, below).
                 event_id UUID NOT NULL,
                 -- Entity name, e.g. \"organization\".
                 entity VARCHAR NOT NULL,
                 -- The record pid — Fluvio partition key (per-record order).
                 entity_pid UUID NOT NULL,
                 -- created / updated / deleted / merged.
                 kind VARCHAR NOT NULL,
                 -- When the change occurred (stamped at enqueue).
                 occurred_at TIMESTAMPTZ NOT NULL,
                 -- Actor (bearer-token user pid), if any.
                 actor VARCHAR NULL,
                 -- Envelope schema version.
                 schema_version INTEGER NOT NULL,
                 -- The full envelope (§4) as JSONB.
                 payload JSONB NOT NULL,
                 -- NULL until the relay ships the row to Fluvio.
                 published_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS event_outbox_event_id \
             ON event_outbox (event_id)",
        )
        .await?;
        // The relay polls only unpublished rows in id order.
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS event_outbox_unpublished \
             ON event_outbox (id) WHERE published_at IS NULL",
        )
        .await?;
        Ok(())
    }

    /// Drop the `event_outbox` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS event_outbox")
            .await?;
        Ok(())
    }
}
