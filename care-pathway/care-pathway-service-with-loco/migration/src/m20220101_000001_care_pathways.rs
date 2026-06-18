//! Migration: create the `care_pathways` table — the entity store. The
//! full `CarePathway` payload lives in `data` (JSONB); `pid` and `name`
//! are denormalised for lookup and listing, and `active` / `deleted_at`
//! drive soft-delete.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The `care_pathways` table migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `care_pathways` table and its columns.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "care_pathways",
            &[
                // Internal auto-increment primary key.
                ("id", ColType::PkAuto),
                // Public, unique, externally exposed UUID.
                ("pid", ColType::UuidUniq),
                // Denormalised name, for lookup/listing/ILIKE search.
                ("name", ColType::String),
                // Full `care_pathway_matcher::CarePathway` payload as JSON.
                ("data", ColType::JsonBinary),
                // Active flag; cleared on soft-delete (defaults true).
                ("active", ColType::BooleanWithDefault(true)),
                // Soft-delete timestamp; null while active.
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `care_pathways` table (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "care_pathways").await?;
        Ok(())
    }
}
