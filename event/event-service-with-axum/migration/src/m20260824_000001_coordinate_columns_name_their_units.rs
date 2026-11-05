//! Migration: rename the coordinate columns so their names carry their
//! units — `latitude` -> `latitude_as_decimal_degrees`, `longitude` ->
//! `longitude_as_decimal_degrees` on `event_locations`.
//!
//! A companion to `m20260822_000001_location_coordinates_to_numeric`,
//! which changed the *type* from double precision to numeric so a
//! coordinate survives a round trip exactly. That migration made the
//! values right; this one makes the names say what they are, so a reader
//! of the schema cannot mistake degrees for radians without opening the
//! spec (`spec/latitude-longitude-as-decimal-degrees`).
//!
//! A **rename**, not a new column plus a copy: metadata-only, so it is
//! fast on a large table and the two names cannot disagree in between.

use sea_orm_migration::prelude::*;

/// The coordinate-column rename (name derived from the module).
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Rename the two columns.
    ///
    /// # Errors
    ///
    /// Propagates any DDL error.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE event_locations
                     RENAME COLUMN latitude TO latitude_as_decimal_degrees;
                 ALTER TABLE event_locations
                     RENAME COLUMN longitude TO longitude_as_decimal_degrees;",
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
                "ALTER TABLE event_locations
                     RENAME COLUMN latitude_as_decimal_degrees TO latitude;
                 ALTER TABLE event_locations
                     RENAME COLUMN longitude_as_decimal_degrees TO longitude;",
            )
            .await?;
        Ok(())
    }
}
