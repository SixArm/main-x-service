//! Migration: outbound webhooks and their delivery log (CMS-R23).
//!
//! Webhooks are the **only** extension mechanism (CMS-D12): declared
//! subscriptions delivered over HTTPS, signed, timed out, size-capped,
//! retried with backoff, and logged. Loading third-party code into a
//! service that forbids `unsafe` and gates every input would forfeit
//! precisely the properties this family exists to demonstrate.
//!
//! Two columns deserve a note:
//!
//! - **`secret`** is stored recoverably, unlike a preview token's hash,
//!   because the receiver must verify the signature and therefore holds
//!   the same secret. It is returned once at registration and never
//!   again by any read.
//! - **`event_kinds`** is JSONB: a subscription names the kinds it
//!   wants, and an empty list means all of them.
//!
//! `webhook_deliveries` is an attempt log, not a queue of intentions:
//! a row exists because a delivery was *tried*, which is what makes
//! "why did our CDN not purge?" answerable.

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
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS webhooks (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 site_pid UUID NOT NULL,
                 name VARCHAR NOT NULL,
                 url VARCHAR NOT NULL,
                 event_kinds JSONB NOT NULL DEFAULT '[]'::jsonb,
                 secret VARCHAR NOT NULL,
                 active BOOLEAN NOT NULL DEFAULT TRUE,
                 last_delivered_at TIMESTAMPTZ NULL,
                 consecutive_failures INTEGER NOT NULL DEFAULT 0,
                 deleted_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE TABLE IF NOT EXISTS webhook_deliveries (
                 created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
                 id SERIAL PRIMARY KEY,
                 pid UUID NOT NULL UNIQUE,
                 webhook_pid UUID NOT NULL,
                 event_id UUID NOT NULL,
                 event_kind VARCHAR NOT NULL,
                 attempt INTEGER NOT NULL DEFAULT 1,
                 state VARCHAR NOT NULL,
                 status_code INTEGER NULL,
                 error VARCHAR NULL,
                 delivered_at TIMESTAMPTZ NULL
             )",
        )
        .await?;
        conn.execute_unprepared("CREATE INDEX IF NOT EXISTS webhooks_site ON webhooks (site_pid)")
            .await?;
        // One attempt row per (webhook, event, attempt): the dedupe key
        // that stops a rerun from re-delivering what already succeeded.
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS webhook_deliveries_unique \
             ON webhook_deliveries (webhook_pid, event_id, attempt)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS webhook_deliveries_webhook \
             ON webhook_deliveries (webhook_pid)",
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
        for table in ["webhook_deliveries", "webhooks"] {
            conn.execute_unprepared(&format!("DROP TABLE IF EXISTS {table}"))
                .await?;
        }
        Ok(())
    }
}
