//! Migration: normalize legacy capitalized `workers.gender` values to
//! the lowercase vocabulary the CHECK constraint admits.
//!
//! A **data** migration, not a schema one: it repairs rows written by
//! the pre-2026-07-23 repository layer, which persisted the domain
//! enum's bare `Debug` form ("Male"). On a constrained schema those
//! writes were rejected outright, so this is a no-op there; it matters
//! for deployments whose `workers` table lacks the constraint. Wraps
//! the hand-written SQL under
//! `../../migrations/2026072300000002_normalize_worker_gender_case/{up,down}.sql`
//! via `include_str!`, matching this crate's other migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m20260723_000002_normalize_worker_gender_case"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072300000002_normalize_worker_gender_case/up.sql"
            ))
            .await?;
        Ok(())
    }

    /// Intentionally a no-op — the normalization is one-way (see the
    /// `down.sql` comment: re-capitalizing would both violate the CHECK
    /// constraint and corrupt rows that were always lowercase).
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(include_str!(
                "../../migrations/2026072300000002_normalize_worker_gender_case/down.sql"
            ))
            .await?;
        Ok(())
    }
}
