//! Migration: drop the never-used `organizations` table cluster
//! (`organizations`, `organization_addresses`, `organization_contacts`,
//! `organization_identifiers`), created by
//! `m20241228_000001_create_organizations`.
//!
//! Spec §13 T-14: `organizations` was wired only as a `belongs_to`
//! foreign-key target from its own three child tables — nothing in the
//! service ever inserted a row (`grep -rn "organizations::ActiveModel\|
//! organizations::Entity::insert" src/db/*.rs` returns zero hits), so
//! the table was migrated and permanently empty in every deployment.
//! The domain model (`crate::models::Organization`) and the `SeaORM`
//! entities are removed in the same change (T-14); this migration
//! removes the schema they backed.
//!
//! A **new** migration rather than editing
//! `m20241228_000001_create_organizations` in place — that migration's
//! own `down.sql` already reverses it correctly, but rewriting a past
//! migration is how a database that already ran it and one that never
//! did disagree about what "the schema" is. `down()` here recreates the
//! four tables verbatim from the original `up.sql` (indexes included),
//! so the migration is reversible exactly like every other one in this
//! migrator.

use sea_orm_migration::prelude::*;

/// The organizations-cluster drop (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Drop the four dead tables, children first.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "DROP TABLE IF EXISTS organization_contacts;
                 DROP TABLE IF EXISTS organization_addresses;
                 DROP TABLE IF EXISTS organization_identifiers;
                 DROP TABLE IF EXISTS organizations CASCADE;",
            )
            .await?;
        Ok(())
    }

    /// Recreate the four tables exactly as
    /// `m20241228_000001_create_organizations`'s `up.sql` did.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2024122800000001_create_organizations/up.sql"
            ))
            .await?;
        Ok(())
    }
}
