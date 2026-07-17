//! Migration: create the `stays` and `transfers` tables — the inpatient
//! episode (admission → transfers → discharge) with the SAFER fields
//! (`senior_review_at`, `edd`, `ccd`, `ccd_met`) and the DTOC clock
//! anchor (`discharge_ready_at`), plus the immutable per-move record.

use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

/// The stays/transfers migration (name derived from the module path).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Create `stays` and `transfers`.
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "stays",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                // The patient: `person:<pid>` EntityRef URN. Never raw
                // demographics.
                ("person_ref", ColType::String),
                // Denormalised display-name cache (refreshable, maskable).
                ("display_name", ColType::String),
                // admitted | discharge_ready | discharged.
                ("status", ColType::String),
                ("admitted_at", ColType::TimestampWithTimeZone),
                // ed | elective | transfer_in | virtual_admission.
                ("source", ColType::String),
                // Current location; bed is null only on a virtual ward
                // before slot placement (normally set).
                ("ward_pid", ColType::UuidNull),
                ("bed_pid", ColType::UuidNull),
                // Virtual-ward stays: where the patient is at home.
                ("home_location_note", ColType::StringNull),
                ("named_nurse_ref", ColType::StringNull),
                ("consultant_ref", ColType::StringNull),
                // SAFER "S": last senior review.
                ("senior_review_at", ColType::TimestampWithTimeZoneNull),
                // SAFER "A": expected discharge date.
                ("edd", ColType::DateNull),
                // Clinical criteria for discharge (free text) + met flag.
                ("ccd", ColType::StringNull),
                ("ccd_met", ColType::BooleanWithDefault(false)),
                // p0 | p1 | p2 | p3 (discharge-to-assess pathways).
                ("discharge_pathway", ColType::StringNull),
                // Start of any DTOC clock.
                ("discharge_ready_at", ColType::TimestampWithTimeZoneNull),
                ("discharged_at", ColType::TimestampWithTimeZoneNull),
                // home | home_with_support | community_hospital | care_home
                // | other_acute | deceased | self_discharge.
                ("discharge_destination", ColType::StringNull),
                // Whiteboard alert chips (JSON array of short strings).
                ("alerts", ColType::JsonBinary),
                ("deleted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        create_table(
            m,
            "transfers",
            &[
                ("id", ColType::PkAuto),
                ("pid", ColType::UuidUniq),
                ("stay_pid", ColType::Uuid),
                // Null on the admission placement.
                ("from_bed_pid", ColType::UuidNull),
                // Null on discharge (the patient leaves the estate).
                ("to_bed_pid", ColType::UuidNull),
                // admission | clinical | capacity | isolation |
                // patient_request | discharge | step_up | step_down.
                ("reason", ColType::String),
                ("moved_at", ColType::TimestampWithTimeZone),
                // `worker:<pid>` actor, when known.
                ("moved_by_ref", ColType::StringNull),
            ],
            &[],
        )
        .await?;
        let conn = m.get_connection();
        // Locate: one active stay per person (partial index; the
        // one-active-stay rule is app-enforced at admission).
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS stays_person ON stays (person_ref) \
             WHERE deleted_at IS NULL",
        )
        .await?;
        // Whiteboard: active stays by ward; occupancy: by bed.
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS stays_ward ON stays (ward_pid) \
             WHERE deleted_at IS NULL AND discharged_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS stays_bed ON stays (bed_pid) \
             WHERE deleted_at IS NULL AND discharged_at IS NULL",
        )
        .await?;
        conn.execute_unprepared(
            "CREATE INDEX IF NOT EXISTS transfers_stay ON transfers (stay_pid)",
        )
        .await?;
        Ok(())
    }

    /// Drop the stays tables (rollback).
    ///
    /// # Errors
    ///
    /// Propagates any `SchemaManager` DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "transfers").await?;
        drop_table(m, "stays").await?;
        Ok(())
    }
}
