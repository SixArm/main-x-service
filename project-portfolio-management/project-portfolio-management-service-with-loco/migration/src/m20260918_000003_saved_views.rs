//! Migration: **`saved_views`** — per-user saved filter/sort/column
//! presets for a front-end route (T-28l). Keyed by the token `sub`
//! (the caller's user pid) and **nothing else identity-shaped**: no
//! email, no name, no session id. A view is always scoped to the
//! `route` it was saved from — the read path never applies one
//! route's view to another.

use loco_rs::schema::{create_table, drop_table, ColType};
use sea_orm_migration::prelude::*;

/// The `saved_views` migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "saved_views",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                // The PASETO `sub` claim — a user pid, never an email
                // or other identity-shaped value.
                ("sub", ColType::String),
                // The front-end route this view was saved from; the
                // read path filters by this, never applying a view
                // saved on one route to another.
                ("route", ColType::String),
                ("name", ColType::String),
                ("filter", ColType::JsonBinary),
                ("sort", ColType::JsonBinary),
                ("columns", ColType::JsonBinary),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        m.get_connection()
            .execute_unprepared(
                "CREATE INDEX saved_views_sub_route ON saved_views (sub, route) \
                     WHERE deleted_at IS NULL;",
            )
            .await?;
        Ok(())
    }

    /// Drop the table.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "saved_views").await
    }
}
