//! Loco migrator for place-service.
//!
//! Each migration wraps the corresponding hand-written SQL under
//! `../migrations/<timestamp>_<name>/{up,down}.sql` via `include_str!`,
//! so the SQL stays the single source of truth while loco gains a Rust
//! `Migrator` it can run (`auto_migrate`, `cargo loco db migrate`).

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;

mod m20260608_000001_create_places;
mod m20260608_000002_create_audit_and_merge;
mod m20260708_000001_create_event_outbox;
mod m20260719_000001_create_review_queue;
mod m20260728_000001_add_integrity_digests;
mod m20260822_000001_geo_coordinates_to_numeric;
mod m20260824_000001_coordinate_columns_name_their_units;

/// The migration runner this crate exposes to loco / `sea-orm-migration`.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260608_000001_create_places::Migration),
            Box::new(m20260608_000002_create_audit_and_merge::Migration),
            Box::new(m20260708_000001_create_event_outbox::Migration),
            Box::new(m20260719_000001_create_review_queue::Migration),
            Box::new(m20260728_000001_add_integrity_digests::Migration),
            Box::new(m20260822_000001_geo_coordinates_to_numeric::Migration),
            Box::new(m20260824_000001_coordinate_columns_name_their_units::Migration),
        ]
    }
}
