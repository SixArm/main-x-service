use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "audit_logs",
            &[
                ("id", ColType::PkAuto),
                // The care-pathway pid the entry concerns.
                ("entity_pid", ColType::Uuid),
                // created / updated / deleted.
                ("action", ColType::String),
                // Optional actor (user id / system).
                ("actor", ColType::StringNull),
                // Snapshot of the record at the time of the action.
                ("snapshot", ColType::JsonBinaryNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "audit_logs").await?;
        Ok(())
    }
}
