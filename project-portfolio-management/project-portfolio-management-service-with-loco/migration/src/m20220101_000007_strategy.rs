//! Migration: the PPM Phase-C **strategy** tables (spec/15-roadmap
//! PPM-2/4/5/11) — `ideas` (the pre-proposal funnel), `scenarios`
//! (what-if portfolios), `objectives` + `objective_links` (OKR
//! alignment), and `benefits` (value realization). Money stays
//! integer minor units + ISO-4217 (the Phase-A posture).

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The strategy-tables migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create the five Phase-C tables.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "ideas",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("title", ColType::String),
                ("pitch", ColType::TextNull),
                // Free-form labels (JSON array of short strings).
                ("tags", ColType::JsonBinary),
                ("votes", ColType::Integer),
                // open | converted | dismissed.
                ("status", ColType::String),
                // Set on convert: the minted proposal.
                ("converted_proposal_pid", ColType::UuidNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "scenarios",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("name", ColType::String),
                ("description", ColType::TextNull),
                // Candidate membership: {"work_item_pids": [...],
                // "proposal_pids": [...]} (JSON).
                ("members", ColType::JsonBinary),
                // Constraint knobs (nullable = unconstrained).
                ("budget_cap_minor", ColType::BigIntegerNull),
                ("currency", ColType::StringNull),
                // Pids that must appear in the membership (JSON array).
                ("must_include", ColType::JsonBinary),
                // draft | committed.
                ("status", ColType::String),
                ("committed_at", ColType::TimestampWithTimeZoneNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "objectives",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("title", ColType::String),
                ("description", ColType::TextNull),
                // e.g. "2026-H2" / "FY27" — a display period, not a date.
                ("period", ColType::StringNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "objective_links",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("objective_pid", ColType::Uuid),
                ("work_item_pid", ColType::Uuid),
                // How strongly the item serves the objective (1–5).
                ("weight", ColType::Integer),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "benefits",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("work_item_pid", ColType::Uuid),
                ("title", ColType::String),
                // cost_saving | revenue | risk_reduction | quality |
                // compliance | other.
                ("category", ColType::String),
                // Financial benefits: target + realized in minor units.
                ("currency", ColType::StringNull),
                ("target_minor", ColType::BigIntegerNull),
                ("realized_minor", ColType::BigInteger),
                // Non-financial benefits: the measure in words.
                ("target_note", ColType::TextNull),
                ("realized_note", ColType::TextNull),
                ("expected_on", ColType::DateNull),
                // planned | on_track | realized | missed.
                ("status", ColType::String),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        let conn = m.get_connection();
        // One mapping per (objective, item); re-linking updates weight.
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS objective_links_pair \
             ON objective_links (objective_pid, work_item_pid)",
        )
        .await?;
        for (index, table, column) in [
            ("objective_links_item", "objective_links", "work_item_pid"),
            ("benefits_item", "benefits", "work_item_pid"),
        ] {
            conn.execute_unprepared(&format!(
                "CREATE INDEX IF NOT EXISTS {index} ON {table} ({column})"
            ))
            .await?;
        }
        Ok(())
    }

    /// Drop the Phase-C tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "benefits").await?;
        drop_table(m, "objective_links").await?;
        drop_table(m, "objectives").await?;
        drop_table(m, "scenarios").await?;
        drop_table(m, "ideas").await?;
        Ok(())
    }
}
