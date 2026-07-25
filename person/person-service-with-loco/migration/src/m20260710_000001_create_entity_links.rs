//! Migration: create the `entity_links` table — the cross-service
//! linking **write side** (`agents/share/cross-service-linking.md` §4.1).
//! For v1 person originates the `same_identity` (person ↔ worker)
//! backbone edge (§9). Wraps the hand-written SQL under
//! `../../migrations/2026071000000001_create_entity_links/{up,down}.sql`
//! via `include_str!`, matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260710_000001_create_entity_links"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026071000000001_create_entity_links/up.sql"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026071000000001_create_entity_links/down.sql"
            ))
            .await?;
        Ok(())
    }
}
