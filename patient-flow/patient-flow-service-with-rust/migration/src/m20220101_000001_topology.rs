//! Migration: create the physical-topology tables — `sites`, `wards`,
//! `bays`, `beds` (spec `domain-model.md`). Beds carry the live state
//! machine columns (`state`, `state_since`, `closure_reason`,
//! `deep_clean_required`) plus the allocation-rule attributes.

use loco_rs::schema::{ColType, create_table, drop_table};
use sea_orm_migration::prelude::*;

/// The topology-tables migration (name derived from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `sites`, `wards`, `bays`, and `beds`.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "sites",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("name", ColType::String),
                // `place:<pid>` EntityRef URN in place-service, if linked.
                ("place_ref", ColType::StringNull),
                // The trust: `organization:<pid>` EntityRef URN.
                ("organization_ref", ColType::StringNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "wards",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("site_pid", ColType::Uuid),
                ("name", ColType::String),
                // Short display code, unique per site (app-enforced).
                ("code", ColType::String),
                // inpatient | assessment | virtual.
                ("kind", ColType::String),
                ("specialty", ColType::StringNull),
                // Closed wards accept no admissions at all.
                ("open", ColType::BooleanWithDefault(true)),
                // Surge-capacity ward, reported separately in capacity.
                ("escalation", ColType::BooleanWithDefault(false)),
                // Outbreak control: existing patients stay, allocator refuses.
                ("closed_to_admissions", ColType::BooleanWithDefault(false)),
                ("place_ref", ColType::StringNull),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "bays",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("ward_pid", ColType::Uuid),
                ("name", ColType::String),
                // male | female | mixed | flexible — allocation rule 2.
                ("sex_designation", ColType::String),
                // Single-occupancy isolation-suited room.
                ("side_room", ColType::BooleanWithDefault(false)),
                ("closed_to_admissions", ColType::BooleanWithDefault(false)),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "beds",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("bay_pid", ColType::Uuid),
                // Display label, unique per bay (app-enforced).
                ("number", ColType::String),
                // available | reserved | occupied | awaiting_clean |
                // cleaning | closed — see `flow::bed_state`.
                ("state", ColType::String),
                // When the current state began (turnaround metrics).
                ("state_since", ColType::TimestampWithTimeZone),
                // infection | maintenance | staffing | other; required
                // (app-enforced) when state = closed.
                ("closure_reason", ColType::StringNull),
                // Set on vacate by an infectious stay; gates re-availability.
                ("deep_clean_required", ColType::BooleanWithDefault(false)),
                ("isolation_capable", ColType::BooleanWithDefault(false)),
                ("oxygen", ColType::BooleanWithDefault(false)),
                ("bariatric", ColType::BooleanWithDefault(false)),
                // Virtual-ward slot: skips the cleaning cycle.
                ("virtual", ColType::BooleanWithDefault(false)),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        // Whiteboard and allocator hot paths.
        let conn = m.get_connection();
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS wards_site ON wards (site_pid) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS bays_ward ON bays (ward_pid) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS beds_bay ON beds (bay_pid) WHERE deleted_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS beds_state ON beds (state) WHERE deleted_at IS NULL",
        )
        .await?;
        Ok(())
    }

    /// Drop the topology tables (rollback), children first.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "beds").await?;
        drop_table(m, "bays").await?;
        drop_table(m, "wards").await?;
        drop_table(m, "sites").await?;
        Ok(())
    }
}
