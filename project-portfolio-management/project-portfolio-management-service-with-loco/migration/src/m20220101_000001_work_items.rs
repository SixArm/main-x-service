//! Migration: create the `work_items` table — the registry of portfolio
//! work-item records across the four collections (portfolio / project /
//! product / program). The full `project_portfolio_management_matcher::WorkItem` payload is
//! stored as JSONB in `data`; `pid`, `kind`, `name`, and `portfolio_pid`
//! are denormalised for scoping, lookup, and listing.
//!
//! The four REST collections share this one table; a row's `kind` column
//! is the collection it belongs to, and matching/dedup/merge are always
//! scoped to a single `kind` (the matcher's kind gate is the underlying
//! guarantee). `portfolio_pid` is the denormalised parent link for the
//! child kinds (project / product / program), enabling cheap roll-up of
//! a portfolio's children.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The `work_items`-table migration. `DeriveMigrationName` derives the
/// name from the module path (the timestamped file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `work_items` table.
    ///
    /// # Errors
    ///
    /// When the `CREATE TABLE` fails (e.g. the table already exists).
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "work_items",
            &[
                // Internal auto-increment primary key.
                ("id", ColType::PkAuto),
                // Public, externally exposed UUID; unique.
                ("pid", ColType::UuidUniq),
                // The collection this row belongs to: Portfolio / Project
                // / Product / Program (the `WorkItemKind`). Scopes every
                // query so matching never crosses collections.
                ("kind", ColType::String),
                // Denormalised name, for lookup / listing / ILIKE search.
                ("name", ColType::String),
                // Full `project_portfolio_management_matcher::WorkItem` payload as JSON.
                ("data", ColType::JsonBinary),
                // Denormalised parent portfolio pid (child kinds only;
                // null for the Portfolio kind), for child roll-up.
                ("portfolio_pid", ColType::UuidNull),
                // Active flag; cleared on soft-delete.
                ("active", ColType::BooleanWithDefault(true)),
                // Soft-delete timestamp; null while active.
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    /// Drop the `work_items` table (rollback).
    ///
    /// # Errors
    ///
    /// When the `DROP TABLE` fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "work_items").await?;
        Ok(())
    }
}
