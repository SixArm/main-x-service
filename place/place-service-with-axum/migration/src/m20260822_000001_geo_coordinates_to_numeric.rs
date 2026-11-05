//! Migration: geo coordinates from `DOUBLE PRECISION` to `NUMERIC`.
//!
//! A coordinate is a decimal quantity, not a binary one. `DOUBLE
//! PRECISION` cannot hold `40.7829` — it holds `40.78289999999999793…` —
//! and cannot distinguish it from `40.78290000000000001`. `NUMERIC`
//! stores the digits the caller sent.
//!
//! Unlike event-service's equivalent change, this one fixes no
//! deserialization break: place-service has no internally-tagged enum or
//! flattened struct in its request path, so its `f64` coordinates
//! survived `serde_json`'s `arbitrary_precision` feature. This is the
//! same correctness argument applied for its own sake.
//!
//! `USING` is an exact widening — every double has a `NUMERIC` form — so
//! this migration loses nothing. Existing rows keep the float artefacts
//! they were written with; only values written from here on are exact.
//!
//! `idx_places_geo` is rebuilt automatically by Postgres as part of the
//! type change.

use sea_orm_migration::prelude::*;

/// The coordinate-type migration (name derived from the file name).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Widen the three coordinate columns to `NUMERIC`.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE places
                 ALTER COLUMN geo_latitude  TYPE NUMERIC USING geo_latitude::NUMERIC,
                 ALTER COLUMN geo_longitude TYPE NUMERIC USING geo_longitude::NUMERIC,
                 ALTER COLUMN geo_elevation TYPE NUMERIC USING geo_elevation::NUMERIC;",
        )
        .await?;
        Ok(())
    }

    /// Narrow the columns back to `DOUBLE PRECISION` (rollback).
    ///
    /// Lossy by nature: a `NUMERIC` carrying more precision than a double
    /// can represent is rounded to the nearest double.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE places
                 ALTER COLUMN geo_latitude  TYPE DOUBLE PRECISION USING geo_latitude::DOUBLE PRECISION,
                 ALTER COLUMN geo_longitude TYPE DOUBLE PRECISION USING geo_longitude::DOUBLE PRECISION,
                 ALTER COLUMN geo_elevation TYPE DOUBLE PRECISION USING geo_elevation::DOUBLE PRECISION;",
        )
        .await?;
        Ok(())
    }
}
