use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
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

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "auth_events").await?;
        Ok(())
    }
}
