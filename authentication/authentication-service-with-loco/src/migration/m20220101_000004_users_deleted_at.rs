//! Add `users.deleted_at` for GDPR Art. 17 erasure (soft delete +
//! anonymisation). When set, the account is treated as gone — `/me`,
//! the data export, and any future read path must reject a deleted user
//! — while the row survives so referential history and the `auth_events`
//! audit trail keep their integrity. The erasure transform also
//! anonymises `email` + `name` to a tombstone (see
//! [`crate::models::users`]); this column records *when* that happened.

use loco_rs::schema::{ColType, add_column, remove_column};
use sea_orm_migration::prelude::*;

/// The `users.deleted_at`-column migration (name from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Add the nullable `users.deleted_at` column.
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        add_column(m, "users", "deleted_at", ColType::TimestampWithTimeZoneNull).await?;
        Ok(())
    }

    /// Remove the `users.deleted_at` column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL failure from the schema manager.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        remove_column(m, "users", "deleted_at").await?;
        Ok(())
    }
}
