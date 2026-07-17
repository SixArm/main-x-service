//! Migration: create the demand-side tables — `bed_requests` (the
//! queue the allocator serves), `red_green_days` (the per-stay-per-day
//! journey journal), and `infection_flags` (per-stay IPC precautions).

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The demand-tables migration (name derived from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `bed_requests`, `red_green_days`, and `infection_flags`.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "bed_requests",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                // The patient needing a bed: `person:<pid>` URN.
                ("person_ref", ColType::String),
                // ed | elective | ward_transfer | external | virtual_step_up.
                ("origin", ColType::String),
                ("target_ward_pid", ColType::UuidNull),
                ("specialty", ColType::StringNull),
                // emergency | urgent | routine.
                ("priority", ColType::String),
                // Requirement flags (JSON): isolation, side_room, oxygen,
                // bariatric, sex — see `flow::allocation::Requirements`.
                ("requirements", ColType::JsonBinary),
                // open | allocated | fulfilled | cancelled.
                ("status", ColType::String),
                ("allocated_bed_pid", ColType::UuidNull),
                ("requested_at", ColType::TimestampWithTimeZone),
                ("resolved_at", ColType::TimestampWithTimeZoneNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "red_green_days",
            &[
                ("id", ColType::PkAuto),
                ("stay_pid", ColType::Uuid),
                ("day", ColType::Date),
                // red | green — days start red (spec `patient-journey.md`).
                ("classification", ColType::String),
                // Coded delay reasons, ≤ 2 (JSON array of tokens).
                ("delay_reasons", ColType::JsonBinary),
                ("note", ColType::StringNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "infection_flags",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("stay_pid", ColType::Uuid),
                // contact | droplet | airborne | protective.
                ("precaution", ColType::String),
                // e.g. covid-19, c-diff, mrsa, norovirus.
                ("organism", ColType::StringNull),
                // suspected | confirmed | cleared.
                ("status", ColType::String),
                // Allocation-rule input: only side rooms / isolation beds.
                ("requires_side_room", ColType::BooleanWithDefault(false)),
                ("flagged_at", ColType::TimestampWithTimeZone),
                ("cleared_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS bed_requests_status ON bed_requests (status) \
             WHERE deleted_at IS NULL",
        )
        .await?;
        // One journal row per stay per day (same-day edits update it).
        conn.execute_unprepared(
            "CREATE UNIQUE INDEX IF NOT EXISTS red_green_stay_day \
             ON red_green_days (stay_pid, day)",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS infection_flags_stay ON infection_flags (stay_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the demand tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "infection_flags").await?;
        drop_table(m, "red_green_days").await?;
        drop_table(m, "bed_requests").await?;
        Ok(())
    }
}
