//! Migration: the PPM Phase-B **visibility** tables (spec/15-roadmap
//! PPM-6/8/9) — `work_item_dependencies` (finish-start edges with
//! lag), `milestones`, `allocations` (resource capacity), and
//! `report_definitions` (saved reports). The PPM-7 dashboard is a
//! derived read over these plus the Phase-A governance tables — no
//! table of its own.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The visibility-tables migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the four Phase-B tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "work_item_dependencies",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                // Finish-start: the predecessor must finish before the
                // successor starts (plus `lag_days`).
                ("predecessor_pid", ColType::Uuid),
                ("successor_pid", ColType::Uuid),
                ("lag_days", ColType::Integer),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "milestones",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("work_item_pid", ColType::Uuid),
                ("name", ColType::String),
                ("due", ColType::Date),
                ("done", ColType::BooleanWithDefault(false)),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "allocations",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("work_item_pid", ColType::Uuid),
                // `person:` / `worker:` EntityRef URN — a reference,
                // never copied demographics; never a matcher signal.
                ("person_ref", ColType::String),
                ("role", ColType::StringNull),
                // Percent of the person's capacity (1–100).
                ("percent", ColType::Integer),
                ("start_date", ColType::DateNull),
                ("end_date", ColType::DateNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "report_definitions",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("name", ColType::String),
                // The collection the report runs over.
                ("collection", ColType::String),
                // {stage?, status?, name_like?} — see the run endpoint.
                ("filters", ColType::JsonBinary),
                // Projected columns (subset of the documented set).
                ("fields", ColType::JsonBinary),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        let conn = m.get_connection();
        // One edge per ordered pair; cycle prevention is app-level.
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS dependencies_pair \
             ON work_item_dependencies (predecessor_pid, successor_pid)",
        )
        .await?;
        for (index, table) in [
            ("milestones_item", "milestones"),
            ("allocations_item", "allocations"),
        ] {
            conn.execute_unprepared(&format!(
                "CREATE INDEX IF NOT EXISTS {index} ON {table} (work_item_pid)"
            ))
            .await?;
        }
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS allocations_person ON allocations (person_ref) \
             WHERE deleted_at IS NULL",
        )
        .await?;
        Ok(())
    }

    /// Drop the Phase-B tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "report_definitions").await?;
        drop_table(m, "allocations").await?;
        drop_table(m, "milestones").await?;
        drop_table(m, "work_item_dependencies").await?;
        Ok(())
    }
}
