//! Migration: create the `webhook_deliveries` table — the delivery log
//! for the outbound webhook sink (`agents/share/event-bus.md` §12, repo
//! `tasks.md` EV-3, this crate's own T-28m).
//!
//! One row per **delivery attempt-set** (a target URL's full
//! retry-with-backoff sequence for one outbox event), written once the
//! sequence reaches its final outcome — not one row per HTTP attempt.
//! `attempts` is how many were made. This is the operator-facing record
//! of what was (or was not) delivered; it does not gate outbox
//! progression (see [`crate::webhooks`] — webhook delivery is best-effort
//! and never re-blocks the durable-bus row it fanned out from).

use sea_orm_migration::prelude::*;

/// The `webhook_deliveries` table migration (name derived from the
/// module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `webhook_deliveries` plus an index on `event_id` (an
    /// operator looking up "did event X reach its webhooks" is the
    /// primary read).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS webhook_deliveries (
                     created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                     id SERIAL PRIMARY KEY,
                     -- The envelope's event_id (dedup key), so a delivery
                     -- log row is traceable to its event_outbox row.
                     event_id UUID NOT NULL,
                     -- Entity name, e.g. \"plan\".
                     entity VARCHAR NOT NULL,
                     -- created / updated / deleted / merged.
                     kind VARCHAR NOT NULL,
                     -- The configured target URL this row is about.
                     url VARCHAR NOT NULL,
                     -- How many HTTP attempts this delivery took.
                     attempts INTEGER NOT NULL,
                     -- 'delivered' | 'failed' (final outcome only).
                     status VARCHAR NOT NULL,
                     -- The last HTTP status received, if any response came
                     -- back at all (NULL on a pure transport failure).
                     status_code INTEGER NULL,
                     -- The last error message, if the final attempt failed.
                     error VARCHAR NULL
                 );
                 CREATE INDEX IF NOT EXISTS webhook_deliveries_event
                     ON webhook_deliveries (event_id);",
            )
            .await?;
        Ok(())
    }

    /// Drop `webhook_deliveries`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS webhook_deliveries;")
            .await?;
        Ok(())
    }
}
