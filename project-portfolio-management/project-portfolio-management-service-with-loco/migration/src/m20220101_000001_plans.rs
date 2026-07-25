//! Migration: create the `plans` table — the registry of portfolio
//! plan records across the four collections (portfolio / project /
//! product / program). The full `project_portfolio_management_matcher::Plan` payload is
//! stored as JSONB in `data`; `pid`, `kind`, `name`, and `parent_pid`
//! are denormalised for scoping, lookup, and listing.
//!
//! The four REST collections share this one table; a row's `kind` column
//! is the collection it belongs to, and matching/dedup/merge are always
//! scoped to a single `kind` (the matcher's kind gate is the underlying
//! guarantee). `parent_pid` is the denormalised parent link for the
//! child kinds (project / product / program), enabling cheap roll-up of
//! a portfolio's children.

use loco_rs::schema::{create_table, drop_table, ColType};
use sea_orm_migration::prelude::*;

/// The `plans`-table migration. `DeriveMigrationName` derives the
/// name from the module path (the timestamped file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the `plans` table.
    ///
    /// # Errors
    ///
    /// When the `CREATE TABLE` fails (e.g. the table already exists).
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "plans",
            &[
                // Internal auto-increment primary key.
                ("id", ColType::PkAuto),
                // Public, externally exposed UUID; unique.
                ("pid", ColType::UuidUniq),
                // Optional descriptive kind label (Portfolio / Project /
                // Product / Program), or null. Since the four kinds were
                // unified into one recursive plan tree, `kind` no longer
                // scopes queries — it is metadata only.
                ("kind", ColType::StringNull),
                // Denormalised name, for lookup / listing / ILIKE search.
                ("name", ColType::String),
                // Full `project_portfolio_management_matcher::Plan` payload as JSON.
                ("data", ColType::JsonBinary),
                // Denormalised parent plan pid (any plan may contain any
                // other), for child roll-up; null for a root plan.
                ("parent_pid", ColType::UuidNull),
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

    /// Drop the `plans` table (rollback).
    ///
    /// # Errors
    ///
    /// When the `DROP TABLE` fails.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "plans").await?;
        Ok(())
    }
}
