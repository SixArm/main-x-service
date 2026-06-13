use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "merge_records",
            &[
                ("id", ColType::PkAuto),
                // The surviving (main) case pid.
                ("main_pid", ColType::Uuid),
                // The merged-away (duplicate) case pid, now soft-deleted.
                ("duplicate_pid", ColType::Uuid),
                // Optional operator-supplied reason.
                ("reason", ColType::StringNull),
                // Optional actor (user id / system) — null until JWT auth.
                ("actor", ColType::StringNull),
                // Snapshot of the duplicate's payload at merge time.
                ("transferred", ColType::JsonBinaryNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "merge_records").await?;
        Ok(())
    }
}
