//! Migration: rename the coordinate columns so their names carry their
//! units.
//!
//! `geo_latitude` -> `geo_latitude_as_decimal_degrees`, and likewise for
//! longitude; `geo_elevation` -> `geo_elevation_as_decimal_metres`.
//!
//! A companion to `m20260822_000001_geo_coordinates_to_numeric`, which
//! changed the *type* from double precision to numeric so a coordinate
//! survives a round trip exactly. That migration made the values right;
//! this one makes the names say what they are, so a reader of the schema
//! cannot mistake degrees for radians, or metres for feet, without
//! opening the spec
//! (`spec/latitude-longitude-as-decimal-degrees`).
//!
//! A **rename**, not a new column plus a copy: the data is untouched and
//! the operation is metadata-only, so it is fast on a large table and
//! carries no risk of the two columns disagreeing.

use sea_orm_migration::prelude::*;

/// The coordinate-column rename (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Rename the three columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE places
                     RENAME COLUMN geo_latitude TO geo_latitude_as_decimal_degrees;
                 ALTER TABLE places
                     RENAME COLUMN geo_longitude TO geo_longitude_as_decimal_degrees;
                 ALTER TABLE places
                     RENAME COLUMN geo_elevation TO geo_elevation_as_decimal_metres;",
            )
            .await?;
        Ok(())
    }

    /// Rename them back.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE places
                     RENAME COLUMN geo_latitude_as_decimal_degrees TO geo_latitude;
                 ALTER TABLE places
                     RENAME COLUMN geo_longitude_as_decimal_degrees TO geo_longitude;
                 ALTER TABLE places
                     RENAME COLUMN geo_elevation_as_decimal_metres TO geo_elevation;",
            )
            .await?;
        Ok(())
    }
}
