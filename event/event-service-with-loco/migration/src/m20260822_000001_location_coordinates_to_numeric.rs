//! Migration: geo coordinates from `DOUBLE PRECISION` to `NUMERIC`.
//!
//! A latitude is a decimal quantity, not a binary one. `DOUBLE
//! PRECISION` cannot hold `37.87` — it holds `37.869999999999997`, and
//! every read back was that value re-rounded for display. `NUMERIC`
//! stores the digits the caller sent, so a coordinate round-trips
//! exactly.
//!
//! The change is also what lets these columns survive `serde_json`'s
//! `arbitrary_precision` feature: [`Location`] is an internally-tagged
//! enum, and under that feature serde buffers the variant's fields, at
//! which point an `f64` field can no longer be deserialized.
//!
//! `USING` is an exact widening — every double has a `NUMERIC` form — so
//! this migration loses nothing. Existing rows keep the float artefacts
//! they were stored with; only values written from here on are exact.
//! Back-filling a "cleaner" number would invent precision the caller
//! never sent.
//!
//! [`Location`]: https://schema.org/Place

use sea_orm_migration::prelude::*;

/// The coordinate-type migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Widen both coordinate columns to `NUMERIC`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE event_locations
                 ALTER COLUMN latitude  TYPE NUMERIC USING latitude::NUMERIC,
                 ALTER COLUMN longitude TYPE NUMERIC USING longitude::NUMERIC;",
        )
        .await?;
        Ok(())
    }

    /// Narrow both columns back to `DOUBLE PRECISION` (rollback).
    ///
    /// Lossy by nature: a `NUMERIC` carrying more precision than a
    /// double can represent is rounded to the nearest double.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE event_locations
                 ALTER COLUMN latitude  TYPE DOUBLE PRECISION USING latitude::DOUBLE PRECISION,
                 ALTER COLUMN longitude TYPE DOUBLE PRECISION USING longitude::DOUBLE PRECISION;",
        )
        .await?;
        Ok(())
    }
}
