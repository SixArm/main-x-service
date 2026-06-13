use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "sessions",
            &[
                ("id", ColType::PkAuto),
                ("jid", ColType::StringUniq),
                ("user_pid", ColType::Uuid),
                ("expires_at", ColType::TimestampWithTimeZone),
                ("revoked_at", ColType::TimestampWithTimeZoneNull),
                ("user_agent", ColType::StringNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "sessions").await?;
        Ok(())
    }
}
