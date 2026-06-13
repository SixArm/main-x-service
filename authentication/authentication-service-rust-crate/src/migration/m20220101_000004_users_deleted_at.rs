//! Add `users.deleted_at` for GDPR Art. 17 erasure (soft delete +
//! anonymisation). When set, the account is treated as gone — `/me`,
//! the data export, and any future read path must reject a deleted user
//! — while the row survives so referential history and the `auth_events`
//! audit trail keep their integrity. The erasure transform also
//! anonymises `email` + `name` to a tombstone (see
//! [`crate::models::users`]); this column records *when* that happened.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        add_column(m, "users", "deleted_at", ColType::TimestampWithTimeZoneNull).await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        remove_column(m, "users", "deleted_at").await?;
        Ok(())
    }
}
