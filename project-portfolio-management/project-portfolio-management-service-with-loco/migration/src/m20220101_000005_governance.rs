//! Migration: the PPM Phase-A **governance core** (spec/15-roadmap
//! PPM-1/3/10/12) — `proposals` (work intake), `gate_reviews` +
//! a `plans.stage` column (phase gates), `risks`, and
//! `budget_lines`. All are operational sub-resources: never matcher
//! signals, always audited. Money is stored as integer **minor
//! units** (`*_minor`) plus an ISO-4217 currency code — exact, no
//! floats, no decimal dependency.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The governance-tables migration (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `proposals`, `gate_reviews`, `risks`, `budget_lines`,
    /// and add the operational `stage` column to `plans`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "proposals",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("title", ColType::String),
                ("summary", ColType::TextNull),
                // Which collection an approved proposal becomes.
                ("kind_target", ColType::String),
                // Sponsor EntityRef URN (person: / worker: / organization:).
                ("sponsor_ref", ColType::StringNull),
                ("strategic_rationale", ColType::TextNull),
                // Requested funding, integer minor units + ISO 4217.
                ("requested_minor", ColType::BigIntegerNull),
                ("currency", ColType::StringNull),
                // draft | submitted | in_review | approved | rejected | promoted.
                ("status", ColType::String),
                // Set on promote: the minted plan.
                ("promoted_plan_pid", ColType::UuidNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "gate_reviews",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("plan_pid", ColType::Uuid),
                // g0_concept … g5_benefits (ordered; see governance rules).
                ("gate", ColType::String),
                // approved | approved_with_conditions | hold | rejected.
                ("decision", ColType::String),
                ("conditions", ColType::TextNull),
                // Approver: `worker:<pid>` EntityRef URN.
                ("approver_ref", ColType::StringNull),
                ("decided_at", ColType::TimestampWithTimeZone),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "risks",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("plan_pid", ColType::Uuid),
                ("title", ColType::String),
                ("description", ColType::TextNull),
                // 1–5 each; exposure = probability × impact (derived).
                ("probability", ColType::Integer),
                ("impact", ColType::Integer),
                // open | mitigating | closed | materialised.
                ("status", ColType::String),
                ("owner_ref", ColType::StringNull),
                ("mitigation", ColType::TextNull),
                ("review_date", ColType::DateNull),
                ("escalated_at", ColType::TimestampWithTimeZoneNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "budget_lines",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("plan_pid", ColType::Uuid),
                // capex | opex.
                ("category", ColType::String),
                ("description", ColType::String),
                // ISO 4217, e.g. GBP.
                ("currency", ColType::String),
                ("planned_minor", ColType::BigInteger),
                // Accumulated recorded actuals (minor units).
                ("actual_minor", ColType::BigInteger),
                ("period_start", ColType::DateNull),
                ("period_end", ColType::DateNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        let conn = m.get_connection();
        // Operational stage on the plan itself: the highest gate
        // passed (null until the first approved gate review).
        conn.execute_unprepared("ALTER TABLE plans ADD COLUMN stage VARCHAR NULL")
            .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS proposals_status ON proposals (status) \
             WHERE deleted_at IS NULL",
        )
        .await?;
        for (index, table) in [
            ("gate_reviews_item", "gate_reviews"),
            ("risks_item", "risks"),
            ("budget_lines_item", "budget_lines"),
        ] {
            conn.execute_unprepared(&format!(
                "CREATE INDEX IF NOT EXISTS {index} ON {table} (plan_pid)"
            ))
            .await?;
        }
        Ok(())
    }

    /// Drop the governance tables and the `stage` column (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("ALTER TABLE plans DROP COLUMN IF EXISTS stage")
            .await?;
        drop_table(m, "budget_lines").await?;
        drop_table(m, "risks").await?;
        drop_table(m, "gate_reviews").await?;
        drop_table(m, "proposals").await?;
        Ok(())
    }
}
