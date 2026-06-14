//! Create the `auth_events` table — the durable authentication audit
//! trail (one row per signup / magic-link request / redemption / signout
//! / me). Columns capture *what happened*, the *subject* (email / pid)
//! when known, and an *outcome* `detail`; never a token or secret.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The `auth_events`-table migration (name derived from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `auth_events` table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "auth_events",
            &[
                ("id", ColType::PkAuto),
                // What happened: signup / magic_link_requested /
                // magic_link_redeemed / signout / me.
                ("event", ColType::String),
                // Normalised email when applicable; null otherwise.
                ("email", ColType::StringNull),
                // Subject user pid when known; null otherwise.
                ("user_pid", ColType::UuidNull),
                // Outcome detail, e.g. rate_limited / unknown_email /
                // expired_token. Never tokens or secrets.
                ("detail", ColType::StringNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `auth_events` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "auth_events").await?;
        Ok(())
    }
}
