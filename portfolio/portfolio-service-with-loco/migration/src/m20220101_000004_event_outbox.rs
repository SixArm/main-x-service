//! Migration: create the `event_outbox` table — the transactional-outbox
//! hand-off buffer for the durable event bus (Phase 2;
//! `agents/share/event-bus.md` §3). One row is written **in the same
//! transaction** as the entity mutation, so a committed change always has
//! its event and vice versa; a relay worker (Phase 3, roadmap) later
//! drains unpublished rows to Fluvio and stamps `published_at`.

use loco_rs::schema::*;
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
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "event_outbox",
            &[
                // Auto pk doubles as the global relay order (ORDER BY id).
                ("id", ColType::PkAuto),
                // Envelope id — consumer dedup key (unique, below).
                ("event_id", ColType::Uuid),
                // Entity name, e.g. "work_item".
                ("entity", ColType::String),
                // The record pid — Fluvio partition key (per-record order).
                ("entity_pid", ColType::Uuid),
                // created / updated / deleted / merged.
                ("kind", ColType::String),
                // When the change occurred (stamped at enqueue).
                ("occurred_at", ColType::TimestampWithTimeZone),
                // Actor (bearer-token user pid), if any.
                ("actor", ColType::StringNull),
                // Envelope schema version.
                ("schema_version", ColType::Integer),
                // The full envelope (§4) as JSONB.
                ("payload", ColType::JsonBinary),
                // NULL until the relay ships the row to Fluvio.
                ("published_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        m.get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX IF NOT EXISTS event_outbox_event_id \
                 ON event_outbox (event_id)",
            )
            .await?;
        // The relay polls only unpublished rows in id order.
        m.get_connection()
            .execute_unprepared(
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
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "event_outbox").await?;
        Ok(())
    }
}
