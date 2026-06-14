//! Create the `users` table backing passwordless magic-link accounts.
//!
//! `email` and `api_key` are `UNIQUE`. The password/reset/verification
//! columns are loco scaffolding; the passwordless flow relies on the
//! `magic_link_token` + `magic_link_expiration` pair (short-lived,
//! single-use). The GDPR `deleted_at` column is added later by
//! `m20220101_000004_users_deleted_at`.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The `users`-table migration (name derived from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `users` table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "users",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::Uuid),
                ("email", ColType::StringUniq),
                ("password", ColType::String),
                ("api_key", ColType::StringUniq),
                ("name", ColType::String),
                ("reset_token", ColType::StringNull),
                ("reset_sent_at", ColType::TimestampWithTimeZoneNull),
                ("email_verification_token", ColType::StringNull),
                (
                    "email_verification_sent_at",
                    ColType::TimestampWithTimeZoneNull,
                ),
                ("email_verified_at", ColType::TimestampWithTimeZoneNull),
                ("magic_link_token", ColType::StringNull),
                ("magic_link_expiration", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `users` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "users").await?;
        Ok(())
    }
}
