//! Create the `auth_rate_limits` table — the durable, multi-instance
//! backing store for the magic-link issuance rate limiter (spec §7).
//!
//! One row is recorded per *allowed* issuance request, keyed by the
//! normalised email (`email_key`) with the wall-clock `requested_at`. The
//! limiter prunes rows older than the window, counts the rest, and inserts
//! a new row only when the count is under the cap — all under a per-key
//! advisory lock so concurrent checks for the same email are exact. This
//! replaces the former process-local in-memory limiter, so the quota is now
//! shared across horizontally-scaled instances.
//!
//! (`create_table` also adds loco's conventional `created_at`/`updated_at`
//! audit columns; the limiter ignores them and uses `requested_at`, which it
//! sets explicitly so tests can inject a synthetic "now".)

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The `auth_rate_limits`-table migration (name derived from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `auth_rate_limits` table and a composite
    /// `(email_key, requested_at)` index — the access pattern of every
    /// limiter check (prune + count + min, all scoped to one key).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "auth_rate_limits",
            &[
                ("id", ColType::PkAuto),
                ("email_key", ColType::String),
                ("requested_at", ColType::TimestampWithTimeZone),
            ],
            &[],
        )
        .await?;
        m.create_index(
            Index::create()
                .name("idx_auth_rate_limits_key_requested_at")
                .table(Alias::new("auth_rate_limits"))
                .col(Alias::new("email_key"))
                .col(Alias::new("requested_at"))
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    /// Drop the `auth_rate_limits` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "auth_rate_limits").await?;
        Ok(())
    }
}
